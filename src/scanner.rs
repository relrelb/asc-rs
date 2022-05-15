use std::iter::Peekable;
use std::str::CharIndices;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TokenKind {
    // Single-characters.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Percent,
    Plus,
    Semicolon,
    Slash,
    Star,

    // Literals.
    NumberLiteral,
    StringLiteral,
    Identifier,

    // End-of-file.
    Eof,
}

#[derive(Debug)]
pub struct Token<'a> {
    kind: TokenKind,
    source: &'a str,
    line: usize,
    column: usize,
}

impl Token<'_> {
    pub fn invalid() -> Self {
        Self {
            kind: TokenKind::Eof,
            source: "",
            line: 0,
            column: 0,
        }
    }

    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    pub fn source(&self) -> &str {
        self.source
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }
}

pub struct Scanner<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn read_char(&mut self) -> Option<char> {
        let (i, c) = self.chars.next()?;

        self.offset = i;

        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(c)
    }

    fn read_char_skip_spaces(&mut self) -> Option<char> {
        loop {
            let c = self.read_char()?;
            if !c.is_ascii_whitespace() {
                return Some(c);
            }
        }
    }

    fn read_number_literal(&mut self) -> Result<TokenKind, CompileError> {
        // TODO: Support decimal dot and exponent notation.
        while let Some((_, '0'..='9')) = self.chars.peek() {
            self.read_char();
        }
        Ok(TokenKind::NumberLiteral)
    }

    fn read_string_literal(&mut self, quote: char) -> Result<TokenKind, CompileError> {
        let line = self.line;
        let column = self.column;
        loop {
            match self.read_char() {
                // TODO: Support escaping.
                Some(c) if c == quote => break,
                Some(_) => {}
                None => {
                    return Err(CompileError {
                        message: format!("Unclosed string"),
                        line,
                        column,
                    })
                }
            }
        }
        Ok(TokenKind::StringLiteral)
    }

    fn read_identifier(&mut self) -> Result<TokenKind, CompileError> {
        while let Some((_, 'A'..='Z' | 'a'..='z' | '0'..='9')) = self.chars.peek() {
            self.read_char();
        }
        Ok(TokenKind::Identifier)
    }

    pub fn read_token(&mut self) -> Result<Token<'a>, CompileError> {
        let c = self.read_char_skip_spaces();
        let start = self.offset;
        let line = self.line;
        let column = self.column - 1;
        let kind = match c {
            None => TokenKind::Eof,
            Some('(') => TokenKind::LeftParen,
            Some(')') => TokenKind::RightParen,
            Some('{') => TokenKind::LeftBrace,
            Some('}') => TokenKind::RightBrace,
            Some(',') => TokenKind::Comma,
            Some('.') => TokenKind::Dot,
            Some('-') => TokenKind::Minus,
            Some('%') => TokenKind::Percent,
            Some('+') => TokenKind::Plus,
            Some(';') => TokenKind::Semicolon,
            Some('/') => TokenKind::Slash,
            Some('*') => TokenKind::Star,
            Some('0'..='9') => self.read_number_literal()?,
            Some(quote @ ('"' | '\'')) => self.read_string_literal(quote)?,
            Some('A'..='Z' | 'a'..='z') => self.read_identifier()?,
            Some(c) => {
                return Err(CompileError {
                    message: format!("Unknown character '{}'", c),
                    line,
                    column,
                })
            }
        };
        let end = (self.offset + 1).min(self.source.len());
        let source = &self.source[start..end];
        Ok(Token {
            kind,
            source,
            line,
            column,
        })
    }
}

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}
