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
    // Unary,
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
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            scanner: Scanner::new(source),
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<(), CompileError> {
        let token = self.scanner.read_token()?;
        if token.kind() == kind {
            Ok(())
        } else {
            Err(CompileError {
                message: message.to_string(),
                line: token.line(),
                column: token.column(),
            })
        }
    }

    fn grouping(&mut self) -> Result<(), CompileError> {
        self.compile()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")
    }

    fn number(&mut self, token: Token) {
        println!("Push {}", token.source());
    }

    fn infix(&mut self, token: Token) -> Result<(), CompileError> {
        let next_precedence = match token.kind() {
            TokenKind::Plus => Precedence::Factor,
            TokenKind::Minus => Precedence::Factor,
            TokenKind::Star => Precedence::Primary,
            TokenKind::Slash => Precedence::Primary,
            _ => unreachable!(),
        };
        self.compile_with_precedence(next_precedence)?;

        match token.kind() {
            TokenKind::Plus => println!("Add"),
            TokenKind::Minus => println!("Sub"),
            TokenKind::Star => println!("Mul"),
            TokenKind::Slash => println!("Div"),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn compile_with_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let token = self.scanner.read_token()?;
        match token.kind() {
            TokenKind::LeftParen => self.grouping()?,
            // TokenKind::Minus => self.negate(),
            TokenKind::NumberLiteral => self.number(token),
            // TokenKind::StringLiteral => self.string(),
            TokenKind::Eof => return Ok(()),
            _ => {
                return Err(CompileError {
                    message: format!("Unexpected token: \"{}\"", token.source()),
                    line: token.line(),
                    column: token.column(),
                })
            }
        }

        loop {
            // TODO: Peek token.
            let token = self.scanner.read_token()?;
            if precedence > Precedence::from(token.kind()) {
                break;
            }
            self.infix(token)?;
        }

        Ok(())
    }

    pub fn compile(&mut self) -> Result<(), CompileError> {
        self.compile_with_precedence(Precedence::Assignment)
    }
}
