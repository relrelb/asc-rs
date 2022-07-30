use std::iter::Peekable;
use std::str::CharIndices;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TokenKind {
    // Operators.
    LeftParen,        // (
    RightParen,       // )
    LeftBrace,        // }
    RightBrace,       // {
    LeftSquareBrace,  // [
    RightSquareBrace, // ]
    Ampersand,        // &
    Bang,             // !
    Bar,              // |
    BangEqual,        // !=
    Caret,            // ^
    Comma,            // ,
    Dot,              // .
    Equal,            // =
    DoubleEqual,      // ==
    TripleEqual,      // ===
    Greater,          // >
    DoubleGreater,    // >>
    TripleGreater,    // >>>
    GreaterEqual,     // >=
    Less,             // <
    DoubleLess,       // <<
    LessEqual,        // <=
    Minus,            // -
    DoubleMinus,      // --
    Percent,          // %
    Plus,             // +
    DoublePlus,       // ++
    Semicolon,        // ;
    Slash,            // /
    Star,             // *
    Tilda,            // ~

    // Literals.
    False,
    Identifier,
    Null,
    Number,
    String,
    True,
    Undefined,

    // Keywords.
    Else,
    If,
    InstanceOf,
    Throw,
    Trace,
    Typeof,
    Var,
    While,

    // End-of-file.
    Eof,
}

#[derive(Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub source: &'a str,
    pub line: usize,
    pub column: usize,
}

impl Token<'_> {
    pub const INVALID: Self = Self {
        kind: TokenKind::Eof,
        source: "",
        line: 0,
        column: 0,
    };
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
        // This will be kept on EOF.
        self.offset = self.source.len();

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

    fn read_number(&mut self) -> Result<TokenKind, CompileError> {
        // TODO: Support decimal dot and exponent notation.
        while let Some((_, '0'..='9')) = self.chars.peek() {
            self.read_char();
        }
        Ok(TokenKind::Number)
    }

    fn read_string(&mut self, quote: char) -> Result<TokenKind, CompileError> {
        let line = self.line;
        let column = self.column;
        loop {
            match self.read_char() {
                // TODO: Support escaping.
                Some(c) if c == quote => break,
                Some(_) => {}
                None => {
                    return Err(CompileError {
                        message: "Unclosed string".to_string(),
                        line,
                        column,
                    })
                }
            }
        }
        Ok(TokenKind::String)
    }

    fn read_identifier(&mut self) -> &str {
        let start = self.offset;
        while let Some((_, 'A'..='Z' | 'a'..='z' | '0'..='9')) = self.chars.peek() {
            self.read_char();
        }
        let end = (self.offset + 1).min(self.source.len());
        &self.source[start..end]
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
            Some('[') => TokenKind::LeftSquareBrace,
            Some(']') => TokenKind::RightSquareBrace,
            Some('&') => TokenKind::Ampersand,
            Some('!') => match self.chars.peek() {
                Some((_, '=')) => {
                    self.read_char();
                    TokenKind::BangEqual
                }
                _ => TokenKind::Bang,
            },
            Some('|') => TokenKind::Bar,
            Some('^') => TokenKind::Caret,
            Some(',') => TokenKind::Comma,
            Some('.') => TokenKind::Dot,
            Some('=') => match self.chars.peek() {
                Some((_, '=')) => {
                    self.read_char();
                    match self.chars.peek() {
                        Some((_, '=')) => {
                            self.read_char();
                            TokenKind::TripleEqual
                        }
                        _ => TokenKind::DoubleEqual,
                    }
                }
                _ => TokenKind::Equal,
            },
            Some('>') => match self.chars.peek() {
                Some((_, '=')) => {
                    self.read_char();
                    TokenKind::GreaterEqual
                }
                Some((_, '>')) => {
                    self.read_char();
                    match self.chars.peek() {
                        Some((_, '>')) => {
                            self.read_char();
                            TokenKind::TripleGreater
                        }
                        _ => TokenKind::DoubleGreater,
                    }
                }
                _ => TokenKind::Greater,
            },
            Some('<') => match self.chars.peek() {
                Some((_, '=')) => {
                    self.read_char();
                    TokenKind::LessEqual
                }
                Some((_, '<')) => {
                    self.read_char();
                    TokenKind::DoubleLess
                }
                _ => TokenKind::Less,
            },
            Some('-') => match self.chars.peek() {
                Some((_, '-')) => {
                    self.read_char();
                    TokenKind::DoubleMinus
                }
                _ => TokenKind::Minus,
            },
            Some('%') => TokenKind::Percent,
            Some('+') => match self.chars.peek() {
                Some((_, '+')) => {
                    self.read_char();
                    TokenKind::DoublePlus
                }
                _ => TokenKind::Plus,
            },
            Some(';') => TokenKind::Semicolon,
            Some('/') => TokenKind::Slash,
            Some('*') => TokenKind::Star,
            Some('~') => TokenKind::Tilda,
            Some('0'..='9') => self.read_number()?,
            Some(quote @ ('"' | '\'')) => self.read_string(quote)?,
            Some('A'..='Z' | 'a'..='z') => match self.read_identifier() {
                "else" => TokenKind::Else,
                "false" => TokenKind::False,
                "if" => TokenKind::If,
                "instanceof" => TokenKind::InstanceOf,
                "null" => TokenKind::Null,
                "trace" => TokenKind::Trace,
                "throw" => TokenKind::Throw,
                "true" => TokenKind::True,
                "typeof" => TokenKind::Typeof,
                "undefined" => TokenKind::Undefined,
                "var" => TokenKind::Var,
                "while" => TokenKind::While,
                _ => TokenKind::Identifier,
            },
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
