use crate::scanner::{CompileError, Scanner, Token, TokenKind};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    None,
    Assignment,
    // Or,
    // And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equality,
    Comparison,
    BitwiseShift,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl From<TokenKind> for Precedence {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::Dot | TokenKind::LeftSquareBrace => Self::Call,
            TokenKind::Bang
            | TokenKind::Delete
            | TokenKind::Tilda
            | TokenKind::Throw
            | TokenKind::Typeof => Self::Unary,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Self::Factor,
            TokenKind::Plus | TokenKind::Minus => Self::Term,
            TokenKind::DoubleGreater | TokenKind::TripleGreater | TokenKind::DoubleLess => {
                Self::BitwiseShift
            }
            TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::InstanceOf => Self::Comparison,
            TokenKind::BangEqual | TokenKind::DoubleEqual | TokenKind::TripleEqual => {
                Self::Equality
            }
            TokenKind::Ampersand => Self::BitwiseAnd,
            TokenKind::Caret => Self::BitwiseXor,
            TokenKind::Bar => Self::BitwiseOr,
            _ => Self::None,
        }
    }
}

fn property_index(name: &str) -> Option<i32> {
    match name {
        "_x" => Some(0),
        "_y" => Some(1),
        "_xscale" => Some(2),
        "_yscale" => Some(3),
        "_currentframe" => Some(4),
        "_totalframes" => Some(5),
        "_alpha" => Some(6),
        "_visible" => Some(7),
        "_width" => Some(8),
        "_height" => Some(9),
        "_rotation" => Some(10),
        "_target" => Some(11),
        "_framesloaded" => Some(12),
        "_name" => Some(13),
        "_droptarget" => Some(14),
        "_url" => Some(15),
        "_highquality" => Some(16),
        "_focusrect" => Some(17),
        "_soundbuftime" => Some(18),
        "_quality" => Some(19),
        "_xmouse" => Some(20),
        "_ymouse" => Some(21),
        _ => None,
    }
}

struct Compiler<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
    action_data: Vec<u8>,
}

impl<'a> Compiler<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            scanner: Scanner::new(source),
            current: Token::INVALID,
            action_data: Vec::new(),
        }
    }

    fn write_action(&mut self, action: swf::avm1::types::Action) {
        let mut writer = swf::avm1::write::Writer::new(&mut self.action_data, 0);
        writer.write_action(&action).unwrap();
    }

    fn read_token(&mut self) -> Result<Token, CompileError> {
        let next_token = self.scanner.read_token()?;
        let token = std::mem::replace(&mut self.current, next_token);
        Ok(token)
    }

    fn peek_token(&self) -> &Token {
        &self.current
    }

    fn consume(&mut self, kind: TokenKind) -> Result<bool, CompileError> {
        let token = self.peek_token();
        if token.kind == kind {
            self.read_token()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token, CompileError> {
        let token = self.peek_token();
        if token.kind == kind {
            self.read_token()
        } else {
            Err(CompileError {
                message: message.to_string(),
                line: token.line,
                column: token.column,
            })
        }
    }

    fn push(&mut self, value: swf::avm1::types::Value) {
        // TODO: Use constant pool.
        let push = swf::avm1::types::Push {
            values: vec![value],
        };
        self.write_action(swf::avm1::types::Action::Push(push));
    }

    fn grouping(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")?;
        Ok(())
    }

    fn array(&mut self) -> Result<(), CompileError> {
        let mut elements = Vec::new();
        loop {
            if self.consume(TokenKind::RightSquareBrace)? {
                break;
            }
            let element = Vec::new();
            let old_action_data = std::mem::replace(&mut self.action_data, element);
            self.expression()?;
            let element = std::mem::replace(&mut self.action_data, old_action_data);
            elements.push(element);
            if !self.consume(TokenKind::Comma)? {
                self.expect(TokenKind::RightSquareBrace, "Expected ']' after array")?;
                break;
            }
        }
        for element in elements.iter().rev() {
            self.action_data.extend(element);
        }
        self.push(swf::avm1::types::Value::Int(
            elements.len().try_into().unwrap(),
        ));
        self.write_action(swf::avm1::types::Action::InitArray);
        Ok(())
    }

    fn variable_access(
        &mut self,
        name: &str,
        can_assign: bool,
        is_delete: bool,
    ) -> Result<(), CompileError> {
        self.push(swf::avm1::types::Value::Str(name.into()));

        if is_delete && Precedence::from(self.peek_token().kind) < Precedence::Call {
            self.write_action(swf::avm1::types::Action::Delete2);
        } else if can_assign && self.consume(TokenKind::Equal)? {
            self.expression()?;
            self.write_action(swf::avm1::types::Action::SetVariable);
        } else if self.consume(TokenKind::DoublePlus)? {
            self.push(swf::avm1::types::Value::Str(name.into()));
            self.write_action(swf::avm1::types::Action::GetVariable);
            self.write_action(swf::avm1::types::Action::Increment);
            self.write_action(swf::avm1::types::Action::SetVariable);
        } else if self.consume(TokenKind::DoubleMinus)? {
            self.push(swf::avm1::types::Value::Str(name.into()));
            self.write_action(swf::avm1::types::Action::GetVariable);
            self.write_action(swf::avm1::types::Action::Decrement);
            self.write_action(swf::avm1::types::Action::SetVariable);
        } else {
            self.write_action(swf::avm1::types::Action::GetVariable);
        }

        Ok(())
    }

    fn dot(&mut self, can_assign: bool, is_delete: bool) -> Result<(), CompileError> {
        let name = self
            .expect(TokenKind::Identifier, "Expected name")?
            .source
            .to_owned();

        if is_delete && Precedence::from(self.peek_token().kind) < Precedence::Call {
            self.push(swf::avm1::types::Value::Str(name.as_str().into()));
            self.write_action(swf::avm1::types::Action::Delete);
        } else if let Some(property) = property_index(&name) {
            self.push(swf::avm1::types::Value::Int(property));

            if can_assign && self.consume(TokenKind::Equal)? {
                self.expression()?;
                self.write_action(swf::avm1::types::Action::SetProperty);
            } else {
                self.write_action(swf::avm1::types::Action::GetProperty);
            }
        } else {
            self.push(swf::avm1::types::Value::Str(name.as_str().into()));

            if can_assign && self.consume(TokenKind::Equal)? {
                self.expression()?;
                self.write_action(swf::avm1::types::Action::SetMember);
            } else {
                self.write_action(swf::avm1::types::Action::GetMember);
            }
        }

        Ok(())
    }

    fn member_access(&mut self, can_assign: bool, is_delete: bool) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::RightSquareBrace, "Expected ']'")?;

        if is_delete && Precedence::from(self.peek_token().kind) < Precedence::Call {
            self.write_action(swf::avm1::types::Action::Delete);
        } else if can_assign && self.consume(TokenKind::Equal)? {
            self.expression()?;
            self.write_action(swf::avm1::types::Action::SetMember);
        } else {
            self.write_action(swf::avm1::types::Action::GetMember);
        }

        Ok(())
    }

    fn delete(&mut self) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Primary)?;

        while Precedence::from(self.peek_token().kind) >= Precedence::Call {
            let token = self.read_token()?;
            match token.kind {
                TokenKind::Dot => self.dot(false, true)?,
                TokenKind::LeftSquareBrace => self.member_access(false, true)?,
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    fn unary(&mut self, token_kind: TokenKind) -> Result<(), CompileError> {
        match token_kind {
            TokenKind::Minus => self.push(swf::avm1::types::Value::Int(0)),
            TokenKind::Tilda => self.push(swf::avm1::types::Value::Double(u32::MAX.into())),
            _ => {}
        }

        self.expression_with_precedence(Precedence::Unary)?;

        match token_kind {
            TokenKind::Plus => self.write_action(swf::avm1::types::Action::ToNumber),
            TokenKind::Minus => self.write_action(swf::avm1::types::Action::Subtract),
            TokenKind::Tilda => self.write_action(swf::avm1::types::Action::BitXor),
            TokenKind::Bang => self.write_action(swf::avm1::types::Action::Not),
            TokenKind::Throw => self.write_action(swf::avm1::types::Action::Throw),
            TokenKind::Typeof => self.write_action(swf::avm1::types::Action::TypeOf),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn prefix(&mut self, token_kind: TokenKind) -> Result<(), CompileError> {
        let variable = self.expect(TokenKind::Identifier, "Expected variable")?;
        let name = variable.source.to_owned();
        self.push(swf::avm1::types::Value::Str(name.as_str().into()));
        self.push(swf::avm1::types::Value::Str(name.as_str().into()));
        self.write_action(swf::avm1::types::Action::GetVariable);
        match token_kind {
            TokenKind::DoublePlus => self.write_action(swf::avm1::types::Action::Increment),
            TokenKind::DoubleMinus => self.write_action(swf::avm1::types::Action::Decrement),
            _ => unreachable!(),
        }
        self.write_action(swf::avm1::types::Action::SetVariable);

        Ok(())
    }

    fn binary(&mut self, token_kind: TokenKind) -> Result<(), CompileError> {
        let next_precedence = match Precedence::from(token_kind) {
            Precedence::None | Precedence::Primary => unreachable!(),
            Precedence::Assignment => Precedence::BitwiseOr,
            Precedence::BitwiseOr => Precedence::BitwiseXor,
            Precedence::BitwiseXor => Precedence::BitwiseAnd,
            Precedence::BitwiseAnd => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::BitwiseShift,
            Precedence::BitwiseShift => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => unreachable!(),
        };
        self.expression_with_precedence(next_precedence)?;

        match token_kind {
            TokenKind::Ampersand => self.write_action(swf::avm1::types::Action::BitAnd),
            TokenKind::Bar => self.write_action(swf::avm1::types::Action::BitOr),
            TokenKind::Caret => self.write_action(swf::avm1::types::Action::BitXor),
            TokenKind::Percent => self.write_action(swf::avm1::types::Action::Modulo),
            TokenKind::Plus => self.write_action(swf::avm1::types::Action::Add2),
            TokenKind::Minus => self.write_action(swf::avm1::types::Action::Subtract),
            TokenKind::Slash => self.write_action(swf::avm1::types::Action::Divide),
            TokenKind::Star => self.write_action(swf::avm1::types::Action::Multiply),
            TokenKind::DoubleEqual => self.write_action(swf::avm1::types::Action::Equals2),
            TokenKind::TripleEqual => self.write_action(swf::avm1::types::Action::StrictEquals),
            TokenKind::Greater => self.write_action(swf::avm1::types::Action::Greater),
            TokenKind::DoubleGreater => self.write_action(swf::avm1::types::Action::BitRShift),
            TokenKind::TripleGreater => self.write_action(swf::avm1::types::Action::BitURShift),
            TokenKind::GreaterEqual => {
                self.write_action(swf::avm1::types::Action::Less);
                self.write_action(swf::avm1::types::Action::Not);
            }
            TokenKind::Less => self.write_action(swf::avm1::types::Action::Less),
            TokenKind::DoubleLess => self.write_action(swf::avm1::types::Action::BitLShift),
            TokenKind::LessEqual => {
                self.write_action(swf::avm1::types::Action::Greater);
                self.write_action(swf::avm1::types::Action::Not);
            }
            TokenKind::InstanceOf => self.write_action(swf::avm1::types::Action::InstanceOf),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn builtin(
        &mut self,
        action: swf::avm1::types::Action,
        arity: usize,
    ) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '('")?;
        let mut count = 0;
        let token = loop {
            let token = self.peek_token();
            if token.kind == TokenKind::RightParen {
                break self.read_token()?;
            }
            count += 1;
            if count > arity {
                return Err(CompileError {
                    message: format!("Expected {} argument(s), got {}", arity, count),
                    line: token.line,
                    column: token.column,
                });
            }
            self.expression()?;
            if !self.consume(TokenKind::Comma)? {
                break self.expect(TokenKind::RightParen, "Expected ')'")?;
            }
        };
        if count < arity {
            return Err(CompileError {
                message: format!("Expected {} argument(s), got {}", arity, count),
                line: token.line,
                column: token.column,
            });
        }
        self.write_action(action);
        Ok(())
    }

    fn expression_with_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let can_assign = precedence <= Precedence::Assignment;
        let is_delete = precedence == Precedence::Primary;

        let token = self.read_token()?;
        match token.kind {
            TokenKind::LeftParen => self.grouping()?,
            TokenKind::LeftSquareBrace => self.array()?,
            TokenKind::Delete => self.delete()?,
            token_kind @ (TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Tilda
            | TokenKind::Bang
            | TokenKind::Throw
            | TokenKind::Typeof) => self.unary(token_kind)?,
            token_kind @ (TokenKind::DoublePlus | TokenKind::DoubleMinus) => {
                self.prefix(token_kind)?
            }
            TokenKind::Number => {
                let i = token.source.parse().unwrap();
                self.push(swf::avm1::types::Value::Int(i));
            }
            TokenKind::String => {
                let s = token.source[1..token.source.len() - 1].to_owned();
                self.push(swf::avm1::types::Value::Str(s.as_str().into()));
            }
            TokenKind::False => self.push(swf::avm1::types::Value::Bool(false)),
            TokenKind::Null => self.push(swf::avm1::types::Value::Null),
            TokenKind::True => self.push(swf::avm1::types::Value::Bool(true)),
            TokenKind::Undefined => self.push(swf::avm1::types::Value::Undefined),
            TokenKind::Identifier => match token.source {
                "chr" => self.builtin(swf::avm1::types::Action::AsciiToChar, 1)?,
                "getTimer" => self.builtin(swf::avm1::types::Action::GetTime, 0)?,
                "ord" => self.builtin(swf::avm1::types::Action::CharToAscii, 1)?,
                "play" => self.builtin(swf::avm1::types::Action::Play, 0)?,
                "random" => self.builtin(swf::avm1::types::Action::RandomNumber, 1)?,
                "stop" => self.builtin(swf::avm1::types::Action::Stop, 0)?,
                "stopAllSounds" => self.builtin(swf::avm1::types::Action::StopSounds, 0)?,
                variable_name => {
                    let variable_name = variable_name.to_owned();
                    self.variable_access(&variable_name, can_assign, is_delete)?;
                    if is_delete {
                        // Skip invalid delete target check.
                        return Ok(());
                    }
                }
            },
            TokenKind::Eof => {
                return Err(CompileError {
                    message: "Unexpected end of file".to_string(),
                    line: token.line,
                    column: token.column,
                })
            }
            _ => {
                return Err(CompileError {
                    message: format!("Unexpected token: \"{}\"", token.source),
                    line: token.line,
                    column: token.column,
                })
            }
        }

        while Precedence::from(self.peek_token().kind) >= precedence {
            match self.read_token()?.kind {
                TokenKind::Dot => self.dot(can_assign, false)?,
                TokenKind::LeftSquareBrace => self.member_access(can_assign, false)?,
                token_kind => self.binary(token_kind)?,
            }
        }

        if can_assign {
            let token = self.peek_token();
            if token.kind == TokenKind::Equal {
                return Err(CompileError {
                    message: "Invalid assignment target".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }
        }

        if is_delete {
            let token = self.peek_token();
            if Precedence::from(token.kind) < Precedence::Call {
                return Err(CompileError {
                    message: "Invalid delete target".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }
        }

        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Assignment)
    }

    fn trace_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '(' before expression")?;
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")?;
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        self.write_action(swf::avm1::types::Action::Trace);
        Ok(())
    }

    fn variable_declaration(&mut self) -> Result<(), CompileError> {
        let name = self
            .expect(TokenKind::Identifier, "Expected variable name")?
            .source
            .to_owned();
        self.push(swf::avm1::types::Value::Str(name.as_str().into()));
        if self.consume(TokenKind::Equal)? {
            self.expression()?;
            self.write_action(swf::avm1::types::Action::DefineLocal);
        } else {
            self.write_action(swf::avm1::types::Action::DefineLocal2);
        }
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        self.write_action(swf::avm1::types::Action::Pop);
        Ok(())
    }

    fn block_statement(&mut self) -> Result<(), CompileError> {
        while !matches!(
            self.peek_token().kind,
            TokenKind::RightBrace | TokenKind::Eof
        ) {
            self.declaration()?;
        }

        self.expect(TokenKind::RightBrace, "Expected '}' after block")?;
        Ok(())
    }

    fn if_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftParen, "Expected '(' after if")?;
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after condition")?;
        self.write_action(swf::avm1::types::Action::Not);

        let if_body = Vec::new();
        let old_action_data = std::mem::replace(&mut self.action_data, if_body);
        self.statement()?;

        let mut else_body = Vec::new();
        if self.consume(TokenKind::Else)? {
            let if_body = std::mem::replace(&mut self.action_data, else_body);
            self.statement()?;
            else_body = std::mem::replace(&mut self.action_data, if_body);

            self.write_action(swf::avm1::types::Action::Jump(swf::avm1::types::Jump {
                offset: else_body.len().try_into().unwrap(),
            }));
        }

        let if_body = std::mem::replace(&mut self.action_data, old_action_data);
        self.write_action(swf::avm1::types::Action::If(swf::avm1::types::If {
            offset: if_body.len().try_into().unwrap(),
        }));
        self.action_data.extend(if_body);
        self.action_data.extend(else_body);

        Ok(())
    }

    fn while_statement(&mut self) -> Result<(), CompileError> {
        let condition = Vec::new();
        let old_action_data = std::mem::replace(&mut self.action_data, condition);
        self.expect(TokenKind::LeftParen, "Expected '(' after while")?;
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after condition")?;
        self.write_action(swf::avm1::types::Action::Not);

        let body = Vec::new();
        let condition = std::mem::replace(&mut self.action_data, body);
        self.statement()?;
        let body = &self.action_data;
        const JUMP_SIZE: usize = 5;
        self.write_action(swf::avm1::types::Action::Jump(swf::avm1::types::Jump {
            offset: -i16::try_from(condition.len() + body.len() + JUMP_SIZE * 2).unwrap(),
        }));

        let body = std::mem::replace(&mut self.action_data, old_action_data);
        self.action_data.extend(condition);
        self.write_action(swf::avm1::types::Action::If(swf::avm1::types::If {
            offset: body.len().try_into().unwrap(),
        }));
        self.action_data.extend(body);

        Ok(())
    }

    fn statement(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::LeftBrace)? {
            self.block_statement()
        } else if self.consume(TokenKind::If)? {
            self.if_statement()
        } else if self.consume(TokenKind::While)? {
            self.while_statement()
        } else if self.consume(TokenKind::Trace)? {
            self.trace_statement()
        } else {
            self.expression_statement()
        }
    }

    fn declaration(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::Var)? {
            self.variable_declaration()
        } else {
            self.statement()
        }
    }

    fn compile(&mut self) -> Result<(), CompileError> {
        // Initialize `self.current`.
        self.read_token()?;

        while self.peek_token().kind != TokenKind::Eof {
            self.declaration()?;
        }

        Ok(())
    }
}

pub fn compile<W: std::io::Write>(source: &str, output: W) -> Result<(), CompileError> {
    let mut compiler = Compiler::new(source);
    compiler.compile()?;

    const SWF_VERSION: u8 = 32;
    let header = swf::Header {
        compression: swf::Compression::None,
        version: SWF_VERSION,
        stage_size: swf::Rectangle {
            x_min: swf::Twips::new(0),
            x_max: swf::Twips::new(100),
            y_min: swf::Twips::new(0),
            y_max: swf::Twips::new(100),
        },
        frame_rate: swf::Fixed8::ONE,
        num_frames: 0,
    };
    let tags = vec![
        swf::Tag::FileAttributes(swf::FileAttributes::empty()),
        swf::Tag::SetBackgroundColor(swf::Color::from_rgb(0xeeeeee, 255)),
        swf::Tag::DoAction(&compiler.action_data),
        swf::Tag::ShowFrame,
    ];
    swf::write_swf(&header, &tags, output).unwrap();
    Ok(())
}
