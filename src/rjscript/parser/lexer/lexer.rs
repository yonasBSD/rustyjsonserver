use std::{iter::Peekable, str::Chars};

use crate::rjscript::{
    ast::position::Position, parser::{errors::ParseError, lexer::token::{Token, TokenKind}}
};

#[derive(Clone)]
pub struct 
Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    /// Current byte offset into `input`.  Always points to the next `peek`‐able char’s starting byte.
    pos: usize,
    /// If `Some(c)`, that is the current character under consideration; if `None`, we've hit EOF.
    current: Option<char>,
    /// Whether we've already returned EOF.
    finished: bool,
    /// 1-based source line
    line: usize,
    /// 1-based column
    column: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from the full input string.
    pub fn new(input: &'a str) -> Self {
        let mut chars = input.chars().peekable();
        let first = chars.next();
        Lexer {
            input,
            chars,
            pos: 0,
            current: first,
            finished: false,
            line: 1,
            column: 1,
        }
    }

    /// Advance one character by popping from `chars`, updating `pos` and `current`.
    fn advance(&mut self) {
        if let Some(ch) = self.current {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.pos += ch.len_utf8();
        }
        self.current = self.chars.next();
    }

    /// Look at the current character without consuming it.
    fn current_char(&self) -> Option<char> {
        self.current
    }

    /// Peek one character ahead (lookahead) without changing state.
    fn peek_next(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    /// Skip any whitespace until a non-whitespace or EOF.
    /// Skip whitespace and comments (`// ...` and `/* ... */`).
    fn skip_whitespace(&mut self) {
        loop {
            // Skip any whitespace
            while let Some(ch) = self.current_char() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            // Skip line comments
            if self.current_char() == Some('/') && self.peek_next() == Some('/') {
                // consume '//'
                self.advance();
                self.advance();
                // consume until end-of-line or EOF
                let start = self.pos;
                self.scan_while(start, |ch| ch != '\n');
                continue;
            }
            // Skip block comments
            if self.current_char() == Some('/') && self.peek_next() == Some('*') {
                // consume '/*'
                self.advance();
                self.advance();
                // consume until '*/' or EOF
                while let Some(ch) = self.current_char() {
                    if ch == '*' && self.peek_next() == Some('/') {
                        // consume '*/'
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                continue;
            }
            break;
        }
    }

    /// Scan characters while `pred` holds true, returning the slice from `start` to current `pos`.
    fn scan_while<F>(&mut self, start: usize, pred: F) -> &'a str
    where
        F: Fn(char) -> bool,
    {
        while let Some(ch) = self.current_char() {
            if pred(ch) {
                self.advance();
            } else {
                break;
            }
        }
        &self.input[start..self.pos]
    }

    /// Consume an identifier or keyword: [A-Za-z_][A-Za-z0-9_]*
    fn lex_identifier(&mut self) -> String {
        let start = self.pos;
        let slice = self.scan_while(start, |ch| ch.is_ascii_alphanumeric() || ch == '_');
        slice.to_string()
    }

    /// Consume a numeric literal: digits with optional single dot.
    fn lex_number(&mut self) -> Result<f64, ParseError> {
        let start = self.pos;
        let start_pos = Position {
            line: self.line,
            column: self.column,
        };
        // integer part
        self.scan_while(start, |ch| ch.is_ascii_digit());
        // optional fractional part
        if self.current_char() == Some('.') {
            self.advance();
            self.scan_while(self.pos, |ch| ch.is_ascii_digit());
        }
        let text = &self.input[start..self.pos];
        text.parse::<f64>()
            .map_err(|_| ParseError::ExpectedNumber(start_pos))
    }

    fn lex_string(&mut self) -> Result<String, ParseError> {
        // consume the opening quote
        let start_pos = Position {
            line: self.line,
            column: self.column,
        };

        self.advance();
        let mut s = String::new();
        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance(); // consume closing quote
                return Ok(s);
            } else if ch == '\\' {
                self.advance(); // consume '\'
                match self.current_char() {
                    Some('n') => {
                        s.push('\n');
                        self.advance();
                    }
                    Some('t') => {
                        s.push('\t');
                        self.advance();
                    }
                    Some('r') => {
                        s.push('\r');
                        self.advance();
                    }
                    Some('"') => {
                        s.push('"');
                        self.advance();
                    }
                    Some('\\') => {
                        s.push('\\');
                        self.advance();
                    }
                    Some(other) => return Err(ParseError::InvalidEscape(other, start_pos)),
                    None => return Err(ParseError::UnexpectedEOF(start_pos)),
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }
        Err(ParseError::UnterminatedString(start_pos))
    }

    fn lex_template(&mut self) -> Result<String, ParseError> {
        let start_pos = Position {
            line: self.line,
            column: self.column,
        };

        self.advance(); // eat the `
        let mut buf = String::new();
        while let Some(c) = self.current_char() {
            if c == '`' {
                self.advance(); // eat the closing backtick
                return Ok(buf);
            }
            buf.push(c);
            self.advance();
        }

        Err(ParseError::UnterminatedString(start_pos))
    }

    /// Produce the next single `Token`.  On error, returns a `ParseError`.
    pub fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();
        let start_pos = Position {
            line: self.line,
            column: self.column,
        };

        let tok = match self.current_char() {
            None => {
                // EOF
                self.finished = true;
                TokenKind::EOF
            }
            Some(ch) => {
                // Handle multi‐char operators first:
                if ch == '=' {
                    if let Some('=') = self.peek_next() {
                        self.advance();
                        self.advance();
                        TokenKind::EqEq
                    } else {
                        self.advance();
                        TokenKind::Eq
                    }
                } else if ch == '\"' {
                    let s = self.lex_string()?;
                    TokenKind::String(s)
                } else if ch == '`' {
                    let s = self.lex_template()?;
                    TokenKind::Template(s)
                } else if ch == '!' {
                    if let Some('=') = self.peek_next() {
                        self.advance();
                        self.advance();
                        TokenKind::BangEq
                    } else {
                        // standalone '!' is not used in this grammar (we do not have boolean negation),
                        // For now, we treat lone '!' as error.
                        return Err(ParseError::UnexpectedChar('!', start_pos));
                    }
                } else if ch == '<' {
                    if let Some('=') = self.peek_next() {
                        self.advance();
                        self.advance();
                        TokenKind::LtEq
                    } else {
                        self.advance();
                        TokenKind::Lt
                    }
                } else if ch == '>' {
                    if let Some('=') = self.peek_next() {
                        self.advance();
                        self.advance();
                        TokenKind::GtEq
                    } else {
                        self.advance();
                        TokenKind::Gt
                    }
                } else if ch == '&' {
                    if let Some('&') = self.peek_next() {
                        self.advance();
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        return Err(ParseError::UnexpectedChar('&', start_pos));
                    }
                } else if ch == '|' {
                    if let Some('|') = self.peek_next() {
                        self.advance();
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        return Err(ParseError::UnexpectedChar('|', start_pos));
                    }
                }
                else if ch == '+' {
                    self.advance();
                    TokenKind::Plus
                } else if ch == '-' {
                    self.advance();
                    TokenKind::Minus
                } else if ch == '*' {
                    self.advance();
                    TokenKind::Star
                } else if ch == '/' {
                    self.advance();
                    TokenKind::Slash
                } else if ch == '%' {
                    self.advance();
                    TokenKind::Percent
                } else if ch == '(' {
                    self.advance();
                    TokenKind::LParen
                } else if ch == ')' {
                    self.advance();
                    TokenKind::RParen
                } else if ch == '{' {
                    self.advance();
                    TokenKind::LBrace
                } else if ch == '}' {
                    self.advance();
                    TokenKind::RBrace
                } else if ch == '[' {
                    self.advance();
                    TokenKind::LBracket
                } else if ch == ']' {
                    self.advance();
                    TokenKind::RBracket
                } else if ch == ';' {
                    self.advance();
                    TokenKind::Semicolon
                } else if ch == ':' {
                    self.advance();
                    TokenKind::Colon
                } else if ch == '.' {
                    self.advance();
                    TokenKind::Dot
                } else if ch == ',' {
                    self.advance();
                    TokenKind::Comma
                }
                // Numeric literal
                else if ch.is_ascii_digit() {
                    let num = self.lex_number()?;
                    TokenKind::Number(num)
                }
                // --- identifier / keyword / bool
                else if ch.is_ascii_alphabetic() || ch == '_' {
                    let ident = self.lex_identifier();
                    match ident.as_str() {
                        "let" => TokenKind::Let,
                        "return" => TokenKind::Return,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "for" => TokenKind::For,
                        "switch" => TokenKind::Switch,
                        "case" => TokenKind::Case,
                        "default" => TokenKind::Default,
                        "func" => TokenKind::Func,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        "req" => TokenKind::Req,
                        "body" => TokenKind::Body,
                        "params" => TokenKind::Params,
                        "query" => TokenKind::Query,
                        "headers" => TokenKind::Headers,
                        "true" => TokenKind::Bool(true),
                        "false" => TokenKind::Bool(false),
                        "undefined" => TokenKind::Undefined,

                        // type keywords
                        "bool" => TokenKind::BoolType,
                        "num" => TokenKind::NumberType,
                        "str" => TokenKind::StringType,
                        "vec" => TokenKind::VecType,
                        "obj" => TokenKind::ObjType,
                        "any" => TokenKind::AnyType,
                        "Undefined" => TokenKind::UndefinedType,

                        // Just an indentifier
                        _ => TokenKind::Ident(ident),
                    }
                } else {
                    // Any other single character is an error:
                    return Err(ParseError::UnexpectedChar(ch, start_pos));
                }
            }
        };

        Ok(Token {
            kind: tok,
            pos: start_pos,
        })
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        match self.next_token() {
            Ok(tok) => Some(Ok(tok)),
            Err(err) => {
                self.finished = true;
                Some(Err(err))
            }
        }
    }
}
