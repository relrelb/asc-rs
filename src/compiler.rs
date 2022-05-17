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
            TokenKind::Bang => Self::Unary,
            TokenKind::BangEqual => Self::Equality,
            TokenKind::Equal => Self::Assignment,
            TokenKind::EqualEqual => Self::Equality,
            TokenKind::Greater => Self::Comparison,
            TokenKind::GreaterEqual => Self::Comparison,
            TokenKind::Less => Self::Comparison,
            TokenKind::LessEqual => Self::Comparison,
            TokenKind::NumberLiteral => Self::None,
            TokenKind::StringLiteral => Self::None,
            TokenKind::Identifier => Self::None,
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

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<(), CompileError> {
        let token = self.read_token()?;
        if token.kind == kind {
            Ok(())
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
        self.expect(TokenKind::RightParen, "Expected ')' after expression")
    }

    fn literal(&mut self, token: Token) {
        println!("Push {}", token.source);
    }

    fn get_variable(&mut self, token: Token) {
        println!("Push \"{}\"", token.source);
        println!("GetVariable");
    }

    fn unary(&mut self, token: Token) -> Result<(), CompileError> {
        self.parse(Precedence::Unary)?;

        match token.kind {
            TokenKind::Minus => println!("Negate"),
            TokenKind::Bang => println!("Not"),
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
        self.parse(next_precedence)?;

        match token.kind {
            TokenKind::Plus => println!("Add"),
            TokenKind::Minus => println!("Sub"),
            TokenKind::Slash => println!("Div"),
            TokenKind::Star => println!("Mul"),
            TokenKind::EqualEqual => println!("Equals"),
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

    fn parse(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        // TODO: Cannot use `self.read_token()` here because of borrow checker.
        let next_token = self.scanner.read_token()?;
        let token = std::mem::replace(&mut self.current, next_token);
        match token.kind {
            TokenKind::LeftParen => self.grouping()?,
            TokenKind::Minus => self.unary(token)?,
            TokenKind::Bang => self.unary(token)?,
            TokenKind::NumberLiteral => self.literal(token),
            TokenKind::StringLiteral => self.literal(token),
            TokenKind::Identifier => self.get_variable(token),
            TokenKind::Eof => return Ok(()),
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

        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.parse(Precedence::Assignment)
    }

    fn compile(&mut self) -> Result<(), CompileError> {
        // Initialize `self.current`.
        self.read_token()?;

        self.expression()?;
        self.expect(TokenKind::Semicolon, "Expected ';' after expression")?;
        self.expect(TokenKind::Eof, "Expected EOF")
    }
}

pub fn compile(source: &str) -> Result<(), CompileError> {
    Compiler::new(source).compile()
}
