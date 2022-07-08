use crate::scanner::{CompileError, Scanner, Token, TokenKind};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    None,
    Assignment,
    // Or,
    // And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    // Call,
    Primary,
}

impl From<TokenKind> for Precedence {
    fn from(kind: TokenKind) -> Self {
        match kind {
            TokenKind::LeftParen => Self::None,
            TokenKind::RightParen => Self::None,
            TokenKind::LeftBrace => Self::None,
            TokenKind::RightBrace => Self::None,
            TokenKind::Comma => Self::None,
            TokenKind::Dot => Self::None,
            TokenKind::Minus => Self::Term,
            TokenKind::MinusMinus => Self::None,
            TokenKind::Percent => Self::Factor,
            TokenKind::Plus => Self::Term,
            TokenKind::PlusPlus => Self::None,
            TokenKind::Semicolon => Self::None,
            TokenKind::Slash => Self::Factor,
            TokenKind::Star => Self::Factor,
            TokenKind::Tilda => Self::Unary,
            TokenKind::Bang => Self::Unary,
            TokenKind::BangEqual => Self::Equality,
            TokenKind::Equal => Self::None,
            TokenKind::EqualEqual => Self::Equality,
            TokenKind::EqualEqualEqual => Self::Equality,
            TokenKind::Greater => Self::Comparison,
            TokenKind::GreaterEqual => Self::Comparison,
            TokenKind::Less => Self::Comparison,
            TokenKind::LessEqual => Self::Comparison,
            TokenKind::Else => Self::None,
            TokenKind::False => Self::None,
            TokenKind::If => Self::None,
            TokenKind::Identifier => Self::None,
            TokenKind::Null => Self::None,
            TokenKind::Number => Self::None,
            TokenKind::String => Self::None,
            TokenKind::True => Self::None,
            TokenKind::Undefined => Self::None,
            TokenKind::Trace => Self::None,
            TokenKind::Typeof => Self::Unary,
            TokenKind::Var => Self::None,
            TokenKind::Eof => Self::None,
        }
    }
}

struct Compiler<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
    writer: swf::avm1::write::Writer<&'a mut Vec<u8>>,
}

impl<'a> Compiler<'a> {
    fn new(source: &'a str, output: &'a mut Vec<u8>) -> Self {
        Self {
            scanner: Scanner::new(source),
            current: Token::INVALID,
            writer: swf::avm1::write::Writer::new(output, 0),
        }
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

    fn grouping(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")?;
        Ok(())
    }

    fn push(&mut self, value: swf::avm1::types::Value) {
        // TODO: Use constant pool.
        let push = swf::avm1::types::Push {
            values: vec![value],
        };
        let action = swf::avm1::types::Action::Push(push);
        self.writer.write_action(&action).unwrap();
    }

    fn variable_access(&mut self, can_assign: bool, token: Token) -> Result<(), CompileError> {
        println!("Push \"{}\"", token.source);
        if can_assign && self.consume(TokenKind::Equal)? {
            self.expression()?;
            println!("SetVariable");
        } else if self.consume(TokenKind::PlusPlus)? {
            println!("GetVariable");
            println!("Increment");
            println!("SetVariable");
        } else if self.consume(TokenKind::MinusMinus)? {
            println!("GetVariable");
            println!("Decrement");
            println!("SetVariable");
        } else {
            println!("GetVariable");
        }
        Ok(())
    }

    fn unary(&mut self, token: Token) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Unary)?;

        match token.kind {
            TokenKind::Plus => {
                let action = swf::avm1::types::Action::ToNumber;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Minus => println!("Negate"),
            TokenKind::Tilda => println!("BitNot"),
            TokenKind::Bang => {
                let action = swf::avm1::types::Action::Not;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Typeof => {
                let action = swf::avm1::types::Action::TypeOf;
                self.writer.write_action(&action).unwrap();
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn binary(&mut self, token: Token) -> Result<(), CompileError> {
        let next_precedence = match Precedence::from(token.kind) {
            Precedence::None | Precedence::Primary => unreachable!(),
            Precedence::Assignment => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Primary,
        };
        self.expression_with_precedence(next_precedence)?;

        match token.kind {
            TokenKind::Percent => {
                let action = swf::avm1::types::Action::Modulo;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Plus => {
                let action = swf::avm1::types::Action::Add2;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Minus => {
                let action = swf::avm1::types::Action::Subtract;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Slash => {
                let action = swf::avm1::types::Action::Divide;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Star => {
                let action = swf::avm1::types::Action::Multiply;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::EqualEqual => {
                let action = swf::avm1::types::Action::Equals2;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::EqualEqualEqual => {
                let action = swf::avm1::types::Action::StrictEquals;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Greater => {
                let action = swf::avm1::types::Action::Greater;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::GreaterEqual => {
                let action = swf::avm1::types::Action::Less;
                self.writer.write_action(&action).unwrap();

                let action = swf::avm1::types::Action::Not;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::Less => {
                let action = swf::avm1::types::Action::Less;
                self.writer.write_action(&action).unwrap();
            }
            TokenKind::LessEqual => {
                let action = swf::avm1::types::Action::Greater;
                self.writer.write_action(&action).unwrap();

                let action = swf::avm1::types::Action::Not;
                self.writer.write_action(&action).unwrap();
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn expression_with_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let can_assign = precedence <= Precedence::Assignment;

        // TODO: Cannot use `self.read_token()` here because of borrow checker.
        let next_token = self.scanner.read_token()?;
        let token = std::mem::replace(&mut self.current, next_token);
        match token.kind {
            TokenKind::LeftParen => self.grouping()?,
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Tilda
            | TokenKind::Bang
            | TokenKind::Typeof => self.unary(token)?,
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let variable = self.expect(TokenKind::Identifier, "Expected variable")?;
                println!("Push \"{}\"", variable.source);
                println!("GetVariable");
                match token.kind {
                    TokenKind::PlusPlus => println!("Increment"),
                    TokenKind::MinusMinus => println!("Decrement"),
                    _ => unreachable!(),
                }
                println!("SetVariable");
            }
            TokenKind::Number => {
                let i = token.source.parse().unwrap();
                let value = swf::avm1::types::Value::Int(i);
                self.push(value);
            }
            TokenKind::String => {
                let s = &token.source[1..token.source.len() - 1];
                let value = swf::avm1::types::Value::Str(s.into());
                self.push(value);
            }
            TokenKind::False => self.push(swf::avm1::types::Value::Bool(false)),
            TokenKind::Null => self.push(swf::avm1::types::Value::Null),
            TokenKind::True => self.push(swf::avm1::types::Value::Bool(true)),
            TokenKind::Undefined => self.push(swf::avm1::types::Value::Undefined),
            TokenKind::Identifier => self.variable_access(can_assign, token)?,
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
            // TODO: Cannot use `self.read_token()` here because of borrow checker.
            let next_token = self.scanner.read_token()?;
            let token = std::mem::replace(&mut self.current, next_token);

            self.binary(token)?;
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

        let action = swf::avm1::types::Action::Trace;
        self.writer.write_action(&action).unwrap();

        Ok(())
    }

    fn variable_declaration(&mut self) -> Result<(), CompileError> {
        let name = self.expect(TokenKind::Identifier, "Expected variable name")?;
        // TODO: Cannot use `self.literal()` here because of borrow checker.
        println!("Push \"{}\"", name.source);
        if self.consume(TokenKind::Equal)? {
            self.expression()?;
        } else {
            println!("Push undefined");
        }
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;
        println!("SetVariable");
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), CompileError> {
        self.expression()?;
        self.expect(TokenKind::Semicolon, "Expected ';' after statement")?;

        let action = swf::avm1::types::Action::Pop;
        self.writer.write_action(&action).unwrap();

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
        println!("If");
        self.statement()?;
        println!("After If");
        if self.consume(TokenKind::Else)? {
            println!("Else");
            self.statement()?;
            println!("After Else");
        }
        Ok(())
    }

    fn statement(&mut self) -> Result<(), CompileError> {
        if self.consume(TokenKind::LeftBrace)? {
            self.block_statement()
        } else if self.consume(TokenKind::If)? {
            self.if_statement()
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
    let mut action_data = vec![];
    Compiler::new(source, &mut action_data).compile()?;

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
        swf::Tag::DoAction(&action_data),
        swf::Tag::ShowFrame,
    ];
    swf::write_swf(&header, &tags, output).unwrap();
    Ok(())
}
