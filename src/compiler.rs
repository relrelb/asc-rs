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
    token: Token<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            scanner: Scanner::new(source),
            token: Token::invalid(),
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<(), CompileError> {
        let line = self.scanner.line();
        let column = self.scanner.column();
        let token = self.scanner.read_token()?;
        if token.kind() == kind {
            Ok(())
        } else {
            Err(CompileError {
                message: message.to_string(),
                line,
                column,
            })
        }
    }

    fn grouping(&mut self) -> Result<(), CompileError> {
        self.compile()?;
        self.expect(TokenKind::RightParen, "Expected ')' after expression")
    }

    fn number(&mut self) {
        println!("Push {}", self.token.source());
    }

    fn infix(&mut self) -> Result<(), CompileError> {
        let op = self.token.kind();
        let next_precedence = match op {
            TokenKind::Plus => Precedence::Factor,
            TokenKind::Minus => Precedence::Factor,
            TokenKind::Star => Precedence::Primary,
            TokenKind::Slash => Precedence::Primary,
            _ => unreachable!(),
        };
        self.compile_with_precedence(next_precedence)?;

        match op {
            TokenKind::Plus => println!("Add"),
            TokenKind::Minus => println!("Sub"),
            TokenKind::Star => println!("Mul"),
            TokenKind::Slash => println!("Div"),
            _ => unreachable!(),
        }

        Ok(())
    }

    fn compile_with_precedence(&mut self, precedence: Precedence) -> Result<(), CompileError> {
        let line = self.scanner.line();
        let column = self.scanner.column();
        self.token = self.scanner.read_token()?;
        match self.token.kind() {
            TokenKind::LeftParen => self.grouping()?,
            // TokenKind::Minus => self.negate(),
            TokenKind::NumberLiteral => self.number(),
            // TokenKind::StringLiteral => self.string(),
            _ => return Err(CompileError {
                message: format!("Unexpected token: \"{}\"", self.token.source()),
                line,
                column,
            }),
        }

        loop {
            // TODO: Peek token.
            self.token = self.scanner.read_token()?;
            if precedence > Precedence::from(self.token.kind()) {
                break;
            }
            self.infix()?;
        }

        Ok(())
    }

    pub fn compile(&mut self) -> Result<(), CompileError> {
        self.compile_with_precedence(Precedence::Assignment)
    }
}
