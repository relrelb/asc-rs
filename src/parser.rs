use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Debug)]
enum TokenKind {
    LeftParen,
    RightParen,
    Semicolon,
    StringLiteral,
    Identifier,
    Eof,
}

#[derive(Debug)]
pub struct Token<'a> {
    kind: TokenKind,
    source: &'a str,
}

impl Token<'_> {
    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }
}

pub struct Parser<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Parser<'a> {
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

    fn read_string_literal(&mut self, quote: char) -> Result<TokenKind, ParseError> {
        let line = self.line;
        let column = self.column;
        loop {
            match self.read_char() {
                // TODO: Support escaping.
                Some(c) if c == quote => break,
                Some(_) => {}
                None => {
                    return Err(ParseError {
                        message: format!("Unclosed string"),
                        line,
                        column,
                    })
                }
            }
        }
        Ok(TokenKind::StringLiteral)
    }

    fn read_identifier(&mut self) -> Result<TokenKind, ParseError> {
        while let Some((_, 'A'..='Z' | 'a'..='z' | '0'..='9')) = self.chars.peek() {
            self.read_char();
        }
        Ok(TokenKind::Identifier)
    }

    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        let c = self.read_char_skip_spaces();
        let start = self.offset;
        let kind = match c {
            None => TokenKind::Eof,
            Some('(') => TokenKind::LeftParen,
            Some(')') => TokenKind::RightParen,
            Some(';') => TokenKind::Semicolon,
            Some(quote @ ('"' | '\'')) => self.read_string_literal(quote)?,
            Some('A'..='Z' | 'a'..='z') => self.read_identifier()?,
            Some(c) => {
                return Err(ParseError {
                    message: format!("Unknown character '{}'", c),
                    line: self.line,
                    column: self.column,
                })
            }
        };
        let end = self.offset;
        let source = &self.source[start..=end];
        Ok(Token { kind, source })
    }
}

#[derive(Debug)]
pub struct ParseError {
    message: String,
    line: usize,
    column: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}
