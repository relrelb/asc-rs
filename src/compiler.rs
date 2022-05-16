use crate::scanner::{CompileError, Scanner, Token, TokenKind};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Precedence {
    None,
    Assignment,
    // Or,
    // And,
    // Equality,
    // Comparison,
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
            TokenKind::NumberLiteral => Self::None,
            TokenKind::StringLiteral => Self::None,
            TokenKind::Identifier => Self::None,
            TokenKind::Eof => Self::None,
        }
    }
}

pub struct Compiler<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
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

    fn unary(&mut self, token: Token) -> Result<(), CompileError> {
        self.parse(Precedence::Unary)?;

        match token.kind {
            TokenKind::Minus => println!("Negate"),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn infix(&mut self, token: Token) -> Result<(), CompileError> {
        let next_precedence = match token.kind {
            TokenKind::Plus => Precedence::Factor,
            TokenKind::Minus => Precedence::Factor,
            TokenKind::Star => Precedence::Primary,
            TokenKind::Slash => Precedence::Primary,
            _ => unreachable!(),
        };
        self.parse(next_precedence)?;

        match token.kind {
            TokenKind::Plus => println!("Add"),
            TokenKind::Minus => println!("Sub"),
            TokenKind::Star => println!("Mul"),
            TokenKind::Slash => println!("Div"),
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
            TokenKind::NumberLiteral => self.literal(token),
            TokenKind::StringLiteral => self.literal(token),
            TokenKind::Eof => return Ok(()),
            _ => {
                return Err(CompileError {
                    message: format!("Unexpected token: \"{}\"", token.source),
                    line: token.line,
                    column: token.column,
                })
            }
        }

        while Precedence::from(self.current.kind) >= precedence {
            // TODO: Cannot use `self.read_token()` here because of borrow checker.
            let next_token = self.scanner.read_token()?;
            let token = std::mem::replace(&mut self.current, next_token);
            self.infix(token)?;
        }

        Ok(())
    }

    fn expression(&mut self) -> Result<(), CompileError> {
        self.parse(Precedence::Assignment)
    }

    pub fn compile(&mut self) -> Result<(), CompileError> {
        // Initialize `self.current`.
        self.read_token()?;

        self.expression()?;
        self.expect(TokenKind::Semicolon, "Expected ';' after expression")?;
        self.expect(TokenKind::Eof, "Expected EOF")
    }
}
