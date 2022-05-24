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
            TokenKind::Percent => Self::Factor,
            TokenKind::Plus => Self::Term,
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
}

impl<'a> Compiler<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            scanner: Scanner::new(source),
            current: Token::INVALID,
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

    fn literal(&mut self, token: Token) {
        println!("Push {}", token.source);
    }

    fn variable_access(&mut self, can_assign: bool, token: Token) -> Result<(), CompileError> {
        println!("Push \"{}\"", token.source);
        if can_assign && self.consume(TokenKind::Equal)? {
            self.expression()?;
            println!("SetVariable");
        } else {
            println!("GetVariable");
        }
        Ok(())
    }

    fn unary(&mut self, token: Token) -> Result<(), CompileError> {
        self.expression_with_precedence(Precedence::Unary)?;

        match token.kind {
            TokenKind::Plus => println!("ToNumber"),
            TokenKind::Minus => println!("Negate"),
            TokenKind::Tilda => println!("BitNot"),
            TokenKind::Bang => println!("Not"),
            TokenKind::Typeof => println!("Typeof"),
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
            TokenKind::Percent => println!("Mod"),
            TokenKind::Plus => println!("Add"),
            TokenKind::Minus => println!("Sub"),
            TokenKind::Slash => println!("Div"),
            TokenKind::Star => println!("Mul"),
            TokenKind::EqualEqual => println!("Equals"),
            TokenKind::EqualEqualEqual => println!("StrictEquals"),
            TokenKind::Greater => println!("Greater"),
            TokenKind::GreaterEqual => {
                println!("Less");
                println!("Not");
            }
            TokenKind::Less => println!("Less"),
            TokenKind::LessEqual => {
                println!("Greater");
                println!("Not");
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
            TokenKind::Number => self.literal(token),
            TokenKind::String => self.literal(token),
            TokenKind::False => println!("Push false"),
            TokenKind::Null => println!("Push null"),
            TokenKind::True => println!("Push true"),
            TokenKind::Undefined => println!("Push undefined"),
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
        println!("Trace");
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
        println!("Pop");
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

pub fn compile(source: &str) -> Result<(), CompileError> {
    Compiler::new(source).compile()
}
