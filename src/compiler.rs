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

impl TokenKind {
    fn precedence(&self) -> Precedence {
        match self {
            Self::LeftParen | Self::Dot | Self::LeftSquareBrace => Precedence::Call,
            Self::Bang | Self::Delete | Self::Tilda | Self::Throw | Self::Typeof => {
                Precedence::Unary
            }
            Self::Star | Self::Slash | Self::Percent => Precedence::Factor,
            Self::Plus | Self::Minus => Precedence::Term,
            Self::DoubleGreater | Self::TripleGreater | Self::DoubleLess => {
                Precedence::BitwiseShift
            }
            Self::Greater
            | Self::GreaterEqual
            | Self::Less
            | Self::LessEqual
            | Self::InstanceOf => Precedence::Comparison,
            Self::BangEqual | Self::DoubleEqual | Self::TripleEqual => Precedence::Equality,
            Self::Ampersand => Precedence::BitwiseAnd,
            Self::Caret => Precedence::BitwiseXor,
            Self::Bar => Precedence::BitwiseOr,
            _ => Precedence::None,
        }
    }

    fn is_assign(&self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::PlusEqual
                | Self::MinusEqual
                | Self::StarEqual
                | Self::SlashEqual
                | Self::PercentEqual
                | Self::AmpersandEqual
                | Self::BarEqual
                | Self::CaretEqual
                | Self::DoubleGreaterEqual
                | Self::TripleGreaterEqual
                | Self::DoubleLessEqual
        )
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

fn register_index(name: &str) -> Option<u8> {
    name.strip_prefix("register").and_then(|r| r.parse().ok())
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

    fn read_token(&mut self) -> Result<Token<'a>, CompileError> {
        let next_token = self.scanner.read_token()?;
        let token = std::mem::replace(&mut self.current, next_token);
        Ok(token)
    }

    fn peek_token(&self) -> &Token<'a> {
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

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<Token<'a>, CompileError> {
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

    fn comma_separated(&mut self, terminator: TokenKind, arity: usize) -> Result<(), CompileError> {
        let mut count = 0;
        let token = loop {
            let token = self.peek_token();
            if token.kind == terminator {
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
                // TODO: Print exact character.
                break self.expect(terminator, "Expected end of values")?;
            }
        };

        if count < arity {
            return Err(CompileError {
                message: format!("Expected {} argument(s), got {}", arity, count),
                line: token.line,
                column: token.column,
            });
        }

        Ok(())
    }

    fn comma_separated_rev(&mut self, terminator: TokenKind) -> Result<usize, CompileError> {
        let mut values = Vec::new();
        loop {
            let token = self.peek_token();
            if token.kind == terminator {
                self.read_token()?;
                break;
            }

            let value_data = Vec::new();
            let old_action_data = std::mem::replace(&mut self.action_data, value_data);
            self.expression()?;
            let value_data = std::mem::replace(&mut self.action_data, old_action_data);
            values.push(value_data);

            if !self.consume(TokenKind::Comma)? {
                // TODO: Print exact character.
                self.expect(terminator, "Expected end of values")?;
                break;
            }
        }

        for value_data in values.iter().rev() {
            self.action_data.extend(value_data);
        }

        Ok(values.len())
    }

    fn array(&mut self) -> Result<(), CompileError> {
        let count = self.comma_separated_rev(TokenKind::RightSquareBrace)?;
        self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));
        self.write_action(swf::avm1::types::Action::InitArray);
        Ok(())
    }

    fn access(
        &mut self,
        push: impl Fn(&mut Self),
        duplicate: impl Fn(&mut Self),
        get: impl Fn(&mut Self),
        set: impl Fn(&mut Self),
        can_assign: bool,
    ) -> Result<(), CompileError> {
        if can_assign && self.peek_token().kind.is_assign() {
            let token = self.read_token()?;
            if token.kind != TokenKind::Equal {
                duplicate(self);
                push(self);
                get(self);
            }
            self.expression()?;
            match token.kind {
                TokenKind::Equal => {}
                TokenKind::PlusEqual => self.write_action(swf::avm1::types::Action::Add2),
                TokenKind::MinusEqual => self.write_action(swf::avm1::types::Action::Subtract),
                TokenKind::StarEqual => self.write_action(swf::avm1::types::Action::Multiply),
                TokenKind::SlashEqual => self.write_action(swf::avm1::types::Action::Divide),
                TokenKind::PercentEqual => self.write_action(swf::avm1::types::Action::Modulo),
                TokenKind::AmpersandEqual => self.write_action(swf::avm1::types::Action::BitAnd),
                TokenKind::BarEqual => self.write_action(swf::avm1::types::Action::BitOr),
                TokenKind::CaretEqual => self.write_action(swf::avm1::types::Action::BitXor),
                TokenKind::DoubleGreaterEqual => {
                    self.write_action(swf::avm1::types::Action::BitRShift)
                }
                TokenKind::TripleGreaterEqual => {
                    self.write_action(swf::avm1::types::Action::BitURShift)
                }
                TokenKind::DoubleLessEqual => {
                    self.write_action(swf::avm1::types::Action::BitLShift)
                }
                _ => unreachable!(),
            }
            set(self);
        } else if self.consume(TokenKind::DoublePlus)? {
            duplicate(self);
            push(self);
            get(self);
            self.write_action(swf::avm1::types::Action::Increment);
            set(self);
        } else if self.consume(TokenKind::DoubleMinus)? {
            duplicate(self);
            push(self);
            get(self);
            self.write_action(swf::avm1::types::Action::Decrement);
            set(self);
        } else {
            push(self);
            get(self);
        }
        Ok(())
    }

    fn variable_access(
        &mut self,
        name: &str,
        can_assign: bool,
        is_delete: bool,
    ) -> Result<(), CompileError> {
        let register = register_index(name);

        if self.consume(TokenKind::LeftParen)? {
            if register.is_some() {
                // TODO: Tell exact location.
                let token = self.peek_token();
                return Err(CompileError {
                    message: "Cannot call register".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }

            let count = self.comma_separated_rev(TokenKind::RightParen)?;
            self.push(swf::avm1::types::Value::Int(count.try_into().unwrap()));
            self.push(swf::avm1::types::Value::Str(name.into()));
            self.write_action(swf::avm1::types::Action::CallFunction);
        } else if is_delete && self.peek_token().kind.precedence() < Precedence::Call {
            if register.is_some() {
                // TODO: Tell exact location.
                let token = self.peek_token();
                return Err(CompileError {
                    message: "Cannot delete register".to_string(),
                    line: token.line,
                    column: token.column,
                });
            }

            self.push(swf::avm1::types::Value::Str(name.into()));
            self.write_action(swf::avm1::types::Action::Delete2);
        } else {
            let push = |this: &mut Self| match register {
                Some(_) => {}
                None => this.push(swf::avm1::types::Value::Str(name.into())),
            };
            let duplicate = push;
            let get = |this: &mut Self| match register {
                Some(register) => this.push(swf::avm1::types::Value::Register(register)),
                None => this.write_action(swf::avm1::types::Action::GetVariable),
            };
            let set = |this: &mut Self| match register {
                Some(register) => this.write_action(swf::avm1::types::Action::StoreRegister(
                    swf::avm1::types::StoreRegister { register },
                )),
                None => this.write_action(swf::avm1::types::Action::SetVariable),
            };
            self.access(push, duplicate, get, set, can_assign)?;
        }

        Ok(())
    }

    fn dot(&mut self, can_assign: bool, is_delete: bool) -> Result<(), CompileError> {
        let name = self.expect(TokenKind::Identifier, "Expected name")?;

        if is_delete && self.peek_token().kind.precedence() < Precedence::Call {
            // TODO: Error when deleting a property?
            self.push(swf::avm1::types::Value::Str(name.source.into()));
            self.write_action(swf::avm1::types::Action::Delete);
        } else {
            let property = property_index(name.source);
            let push = |this: &mut Self| match property {
                Some(property) => this.push(swf::avm1::types::Value::Int(property)),
                None => this.push(swf::avm1::types::Value::Str(name.source.into())),
            };
            let duplicate = |this: &mut Self| {
                this.write_action(swf::avm1::types::Action::PushDuplicate);
                push(this);
                this.write_action(swf::avm1::types::Action::StackSwap);
            };
            let get = |this: &mut Self| match property {
                Some(_) => this.write_action(swf::avm1::types::Action::GetProperty),
                None => this.write_action(swf::avm1::types::Action::GetMember),
            };
            let set = |this: &mut Self| match property {
                Some(_) => this.write_action(swf::avm1::types::Action::SetProperty),
                None => this.write_action(swf::avm1::types::Action::SetMember),
            };
            self.access(push, duplicate, get, set, can_assign)?;
        }

        Ok(())
    }

    fn member_access(&mut self, can_assign: bool, is_delete: bool) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::RightSquareBrace, "Expected ']'")?;

        if is_delete && self.peek_token().kind.precedence() < Precedence::Call {
            self.write_action(swf::avm1::types::Action::Delete);
        } else {
            // TODO: Fix.
            let push = |_this: &mut Self| {};
            let duplicate = |this: &mut Self| {
                this.write_action(swf::avm1::types::Action::StackSwap);
                this.write_action(swf::avm1::types::Action::PushDuplicate);
                this.write_action(swf::avm1::types::Action::StackSwap);
            };
            let get = |this: &mut Self| this.write_action(swf::avm1::types::Action::GetMember);
            let set = |this: &mut Self| this.write_action(swf::avm1::types::Action::SetMember);
            self.access(push, duplicate, get, set, can_assign)?;
        }

        Ok(())
    }

    fn delete(&mut self) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Primary)?;

        while self.peek_token().kind.precedence() >= Precedence::Call {
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
        let register = register_index(variable.source);

        if let Some(register) = register {
            self.push(swf::avm1::types::Value::Register(register));
        } else {
            self.push(swf::avm1::types::Value::Str(variable.source.into()));
            self.push(swf::avm1::types::Value::Str(variable.source.into()));
            self.write_action(swf::avm1::types::Action::GetVariable);
        }

        match token_kind {
            TokenKind::DoublePlus => self.write_action(swf::avm1::types::Action::Increment),
            TokenKind::DoubleMinus => self.write_action(swf::avm1::types::Action::Decrement),
            _ => unreachable!(),
        }

        if let Some(register) = register {
            self.write_action(swf::avm1::types::Action::StoreRegister(
                swf::avm1::types::StoreRegister { register },
            ));
        } else {
            self.write_action(swf::avm1::types::Action::SetVariable);
        }

        Ok(())
    }

    fn binary(&mut self, token: Token) -> Result<(), CompileError> {
        let next_precedence = match token.kind.precedence() {
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
            Precedence::Unary | Precedence::Call => {
                return Err(CompileError {
                    message: "Expected binary operator".to_string(),
                    line: token.line,
                    column: token.column,
                })
            }
        };
        self.expression_with_precedence(next_precedence)?;

        match token.kind {
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
        self.comma_separated(TokenKind::RightParen, arity)?;
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
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Tilda
            | TokenKind::Bang
            | TokenKind::Throw
            | TokenKind::Typeof => self.unary(token.kind)?,
            TokenKind::DoublePlus | TokenKind::DoubleMinus => self.prefix(token.kind)?,
            TokenKind::Number => {
                let integer = token.source.parse().unwrap();
                self.push(swf::avm1::types::Value::Int(integer));
            }
            TokenKind::String => {
                let string = &token.source[1..token.source.len() - 1];
                self.push(swf::avm1::types::Value::Str(string.into()));
            }
            TokenKind::False => self.push(swf::avm1::types::Value::Bool(false)),
            TokenKind::Null => self.push(swf::avm1::types::Value::Null),
            TokenKind::True => self.push(swf::avm1::types::Value::Bool(true)),
            TokenKind::Undefined => self.push(swf::avm1::types::Value::Undefined),
            TokenKind::Function => self.function_expression()?,
            TokenKind::Identifier => match token.source {
                "call" => self.builtin(swf::avm1::types::Action::Call, 1)?,
                "duplicateMovieClip" => self.builtin(swf::avm1::types::Action::CloneSprite, 3)?,
                "chr" => self.builtin(swf::avm1::types::Action::AsciiToChar, 1)?,
                "eval" => self.builtin(swf::avm1::types::Action::GetVariable, 1)?,
                "getTimer" => self.builtin(swf::avm1::types::Action::GetTime, 0)?,
                "int" => self.builtin(swf::avm1::types::Action::ToInteger, 1)?,
                "length" => self.builtin(swf::avm1::types::Action::StringLength, 1)?,
                "mbchr" => self.builtin(swf::avm1::types::Action::MBAsciiToChar, 1)?,
                "mblength" => self.builtin(swf::avm1::types::Action::MBStringLength, 1)?,
                "mbord" => self.builtin(swf::avm1::types::Action::MBCharToAscii, 1)?,
                "mbsubstring" => self.builtin(swf::avm1::types::Action::MBStringExtract, 3)?,
                "nextFrame" => self.builtin(swf::avm1::types::Action::NextFrame, 0)?,
                "ord" => self.builtin(swf::avm1::types::Action::CharToAscii, 1)?,
                "play" => self.builtin(swf::avm1::types::Action::Play, 0)?,
                "prevFrame" => self.builtin(swf::avm1::types::Action::PreviousFrame, 0)?,
                "random" => self.builtin(swf::avm1::types::Action::RandomNumber, 1)?,
                "stop" => self.builtin(swf::avm1::types::Action::Stop, 0)?,
                "stopAllSounds" => self.builtin(swf::avm1::types::Action::StopSounds, 0)?,
                "stopDrag" => self.builtin(swf::avm1::types::Action::EndDrag, 0)?,
                "targetPath" => self.builtin(swf::avm1::types::Action::TargetPath, 1)?,
                "toggleHighQuality" => self.builtin(swf::avm1::types::Action::ToggleQuality, 0)?,
                variable_name => {
                    self.variable_access(variable_name, can_assign, is_delete)?;
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

        while self.peek_token().kind.precedence() >= precedence {
            let token = self.read_token()?;
            match token.kind {
                TokenKind::Dot => self.dot(can_assign, false)?,
                TokenKind::LeftSquareBrace => self.member_access(can_assign, false)?,
                _ => self.binary(token)?,
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
            if token.kind.precedence() < Precedence::Call {
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
        let variable = self.expect(TokenKind::Identifier, "Expected variable name")?;
        self.push(swf::avm1::types::Value::Str(variable.source.into()));
        if self.consume(TokenKind::Equal)? {
            self.expression()?;
            self.write_action(swf::avm1::types::Action::DefineLocal);
        } else {
            self.write_action(swf::avm1::types::Action::DefineLocal2);
        }
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        Ok(())
    }

    fn function_body(&mut self, name: &str) -> Result<(), CompileError> {
        let mut params = Vec::new();
        self.expect(TokenKind::LeftParen, "Expected '('")?;
        loop {
            if self.consume(TokenKind::RightParen)? {
                break;
            }
            let parameter = self.expect(TokenKind::Identifier, "Expected parameter name")?;
            params.push(parameter.source.into());
            if !self.consume(TokenKind::Comma)? {
                self.expect(TokenKind::RightParen, "Expected ')'")?;
                break;
            }
        }

        self.expect(TokenKind::LeftBrace, "Expected '{'")?;
        let actions = Vec::new();
        let old_action_data = std::mem::replace(&mut self.action_data, actions);
        self.block_statement()?;
        let actions = std::mem::replace(&mut self.action_data, old_action_data);

        self.write_action(swf::avm1::types::Action::DefineFunction(
            swf::avm1::types::DefineFunction {
                name: name.into(),
                params,
                actions: &actions,
            },
        ));
        Ok(())
    }

    fn function_declaration(&mut self) -> Result<(), CompileError> {
        let name = self.expect(TokenKind::Identifier, "Expected function name")?;
        self.function_body(name.source)
    }

    fn function_expression(&mut self) -> Result<(), CompileError> {
        let token = self.peek_token();
        if token.kind == TokenKind::Identifier {
            return Err(CompileError {
                message: "Function expression must be anonymous".to_string(),
                line: token.line,
                column: token.column,
            });
        }
        self.function_body("")
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

    fn try_statement(&mut self) -> Result<(), CompileError> {
        self.expect(TokenKind::LeftBrace, "Expected '{'")?;
        let try_body = Vec::new();
        let old_action_data = std::mem::replace(&mut self.action_data, try_body);
        self.block_statement()?;
        let try_body = std::mem::replace(&mut self.action_data, old_action_data);

        let catch_body = if self.consume(TokenKind::Catch)? {
            self.expect(TokenKind::LeftParen, "Expected '('")?;
            let catch_var = self.expect(TokenKind::Identifier, "Expected catch variable")?;
            self.expect(TokenKind::RightParen, "Expected ')'")?;

            self.expect(TokenKind::LeftBrace, "Expected '{'")?;
            let catch_body = Vec::new();
            let old_action_data = std::mem::replace(&mut self.action_data, catch_body);
            self.block_statement()?;
            let catch_body = std::mem::replace(&mut self.action_data, old_action_data);
            Some((catch_var, catch_body))
        } else {
            None
        };

        let finally_body = if self.consume(TokenKind::Finally)? {
            self.expect(TokenKind::LeftBrace, "Expected '{'")?;
            let finally_body = Vec::new();
            let old_action_data = std::mem::replace(&mut self.action_data, finally_body);
            self.block_statement()?;
            let finally_body = std::mem::replace(&mut self.action_data, old_action_data);
            Some(finally_body)
        } else {
            None
        };

        // TODO: Validate existence of catch/finally?

        self.write_action(swf::avm1::types::Action::Try(swf::avm1::types::Try {
            try_body: &try_body,
            catch_body: catch_body.as_ref().map(|(catch_var, catch_body)| {
                let catch_var = if let Some(register) = register_index(catch_var.source) {
                    swf::avm1::types::CatchVar::Register(register)
                } else {
                    swf::avm1::types::CatchVar::Var(catch_var.source.into())
                };
                (catch_var, catch_body.as_ref())
            }),
            finally_body: finally_body.as_deref(),
        }));
        Ok(())
    }

    fn statement(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::LeftBrace)? {
            self.block_statement()
        } else if self.consume(TokenKind::If)? {
            self.if_statement()
        } else if self.consume(TokenKind::While)? {
            self.while_statement()
        } else if self.consume(TokenKind::Try)? {
            self.try_statement()
        } else if self.consume(TokenKind::Trace)? {
            self.trace_statement()
        } else {
            self.expression_statement()
        }
    }

    fn declaration(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::Var)? {
            self.variable_declaration()
        } else if self.consume(TokenKind::Function)? {
            self.function_declaration()
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
