use std::iter::Peekable;

use crate::rjscript::ast::block::Block;
use crate::rjscript::ast::expr::{Expr, ExprKind};
use crate::rjscript::ast::node::Located;
use crate::rjscript::ast::position::Position;
use crate::rjscript::ast::request::RequestFieldType;
use crate::rjscript::ast::stmt::{Stmt};
use crate::rjscript::parser::errors::ParseError;
use crate::rjscript::parser::lexer::lexer::Lexer;
use crate::rjscript::parser::lexer::token::{Token, TokenKind};
use crate::rjscript::parser::stmt::parse_stmt;
use crate::rjscript::parser::ParseResult;
use crate::rjscript::semantics::types::VarType;

pub struct Parser<'a> {
    pub last_pos: Position,
    pub tokens: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    /// Create a new parser from a raw input string by first tokenizing it.
    pub fn new(input: &'a str) -> Result<Self, ParseError> {
        let lexer = Lexer::new(input);
        Ok(Parser {
            last_pos: Position { line: 1, column: 1 },
            tokens: lexer.peekable(),
        })
    }

    /// Peek at the current token without consuming it.
    pub fn peek(&mut self) -> Result<&Token, ParseError> {
        match self.tokens.peek() {
            Some(Ok(tok)) => Ok(tok),
            Some(Err(err)) => Err(err.clone()),
            None => Err(ParseError::UnexpectedEOF(self.last_pos)),
        }
    }

    /// Peek at the current token kind without consuming it.
    pub fn peek_kind(&mut self) -> Result<&TokenKind, ParseError> {
        Ok(&self.peek()?.kind)
    }

    /// Advance one token and return it.
    pub fn advance(&mut self) -> Result<Token, ParseError> {
        match self.tokens.next().transpose()? {
            Some(tok) => {
                self.last_pos = tok.pos;
                Ok(tok)
            }
            None => Err(ParseError::UnexpectedEOF(self.last_pos)),
        }
    }

    /// If the current token matches the given kind, advance and return true; otherwise return false.
    pub fn match_kind(&mut self, expected: TokenKind) -> Result<bool, ParseError> {
        if *self.peek_kind()? == expected {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Expect a specific kind, or error.
    pub fn expect_kind(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        if self.match_kind(expected.clone())? {
            Ok(())
        } else {
            Err(ParseError::General(
                format!("Expected {:?}, found {:?}", expected, self.peek_kind()?),
                self.last_pos,
            ))
        }
    }

    /// Return true if the next token is EOF, or propagate any lex/parse error.
    pub fn is_at_end(&mut self) -> Result<bool, ParseError> {
        Ok(matches!(self.peek_kind()?, TokenKind::EOF))
    }

    /// Utility: if the current token is an identifier, return its String (and consume it); otherwise Err.
    pub fn consume_ident(&mut self) -> Result<String, ParseError> {
        match self.peek_kind()? {
            TokenKind::Ident(name) => {
                let s = name.clone();
                self.advance()?;
                Ok(s)
            }
            _ => {
                if let TokenKind::EOF = self.peek_kind()? {
                    Err(ParseError::UnexpectedEOF(self.last_pos))
                } else {
                    // If it’s not an identifier, and not EOF, it’s some other char/keyword:
                    let found = format!("{:?}", self.peek_kind()?);
                    Err(ParseError::ExpectedIdentifier(found, self.last_pos))
                }
            }
        }
    }

    /// Utility: if current token is a number, return its f64 (and consume); otherwise Err.
    pub fn consume_number(&mut self) -> Result<f64, ParseError> {
        if let TokenKind::Number(n) = self.peek_kind()? {
            let v = *n;
            self.advance()?;
            Ok(v)
        } else {
            Err(ParseError::ExpectedNumber(self.last_pos))
        }
    }

    pub fn parse_req_access(&mut self, start_pos: Position) -> ParseResult<Expr> {
        // Expect: req.<section>...
        self.expect_kind(TokenKind::Dot)?;
        match self.peek_kind()? {
            TokenKind::Body => {
                self.advance()?; // consume 'body'
                Ok(Located::new(
                    ExprKind::RequestField(RequestFieldType::BodyField),
                    start_pos,
                ))
            }
            TokenKind::Params => {
                self.advance()?; // 'params'
                Ok(Located::new(
                    ExprKind::RequestField(RequestFieldType::ParamField),
                    start_pos,
                ))
            }
            TokenKind::Query => {
                self.advance()?; // consume 'query'
                Ok(Located::new(
                    ExprKind::RequestField(RequestFieldType::QueryField),
                    start_pos,
                ))
            }
            TokenKind::Headers => {
                self.advance()?; // consume 'headers'
                Ok(Located::new(
                    ExprKind::RequestField(RequestFieldType::HeadersField),
                    start_pos,
                ))
            }
            other => Err(ParseError::UnexpectedValueAfterReq(
                format!("{:?}", other),
                start_pos,
            )),
        }
    }

    pub fn parse_type(&mut self) -> ParseResult<VarType> {
        // Peek on the TokenKind
        let tk = self.peek_kind()?.clone();
        match tk {
            TokenKind::NumberType => {
                self.advance()?;
                Ok(VarType::Number)
            }
            TokenKind::BoolType => {
                self.advance()?;
                Ok(VarType::Bool)
            }
            TokenKind::StringType => {
                self.advance()?;
                Ok(VarType::String)
            }
            TokenKind::ObjType => {
                self.advance()?;
                Ok(VarType::Object)
            }
            TokenKind::VecType => {
                self.advance()?; // consume 'vec'
                self.expect_kind(TokenKind::Lt)?; // consume '<'
                let inner = self.parse_type()?;
                self.expect_kind(TokenKind::Gt)?; // consume '>'
                Ok(VarType::Array(Box::new(inner)))
            }
            TokenKind::AnyType => {
                self.advance()?;
                Ok(VarType::Any)
            }
            TokenKind::UndefinedType => {
                self.advance()?;
                Ok(VarType::Undefined)
            }
            _ => Err(ParseError::General(
                format!("Unknown type: {:?}", tk),
                self.last_pos,
            )),
        }
    }

    pub fn parse_vec_type(&mut self) -> ParseResult<VarType> {
        // Peek on the TokenKind
        let tk = self.peek_kind()?.clone();
        match tk {
            TokenKind::NumberType => {
                self.advance()?;
                Ok(VarType::Number)
            }
            TokenKind::BoolType => {
                self.advance()?;
                Ok(VarType::Bool)
            }
            TokenKind::StringType => {
                self.advance()?;
                Ok(VarType::String)
            }
            TokenKind::ObjType => {
                self.advance()?;
                Ok(VarType::Object)
            }
            TokenKind::AnyType => {
                self.advance()?;
                Ok(VarType::Any)
            }
            TokenKind::VecType => {
                self.advance()?; // consume 'vec'
                self.expect_kind(TokenKind::Lt)?; // consume '<'
                let inner = self.parse_vec_type()?;
                self.expect_kind(TokenKind::Gt)?; // consume '>'
                Ok(VarType::Array(Box::new(inner)))
            }
            TokenKind::UndefinedType => Err(ParseError::General(
                "Variables and functions cannot be declared as Undefined type".into(),
                self.last_pos,
            )),
            _ => Err(ParseError::General(
                format!("Unknown type: {:?}", tk),
                self.last_pos,
            )),
        }
    }

    pub fn parse_assignment_type(&mut self) -> ParseResult<VarType> {
        // Peek on the TokenKind
        let tk = self.peek_kind()?.clone();
        match tk {
            TokenKind::NumberType => {
                self.advance()?;
                Ok(VarType::Number)
            }
            TokenKind::BoolType => {
                self.advance()?;
                Ok(VarType::Bool)
            }
            TokenKind::StringType => {
                self.advance()?;
                Ok(VarType::String)
            }
            TokenKind::ObjType => {
                self.advance()?;
                Ok(VarType::Object)
            }
            TokenKind::VecType => {
                self.advance()?; // consume 'vec'
                self.expect_kind(TokenKind::Lt)?; // consume '<'
                let inner = self.parse_vec_type()?;
                self.expect_kind(TokenKind::Gt)?; // consume '>'
                Ok(VarType::Array(Box::new(inner)))
            }
            TokenKind::AnyType => Err(ParseError::General(
                "Only vectors can be declared as any".into(),
                self.last_pos,
            )),
            TokenKind::UndefinedType => Err(ParseError::General(
                "Variables and functions cannot be declared as Undefined type".into(),
                self.last_pos,
            )),
            _ => Err(ParseError::General(
                format!("Unknown type: {:?}", tk),
                self.last_pos,
            )),
        }
    }

    fn parse_script(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while *self.peek_kind()? != TokenKind::EOF {
            stmts.push(parse_stmt(self, true)?);
        }
        Ok(stmts)
    }
}

pub fn parse_script(input: &str) -> ParseResult<Block> {
    let mut parser = Parser::new(input)?;
    let stmts = parser.parse_script()?;
    let block_start = Position { line: 0, column: 0 };
    Ok(Block {
        stmts,
        pos: block_start,
    })
}
