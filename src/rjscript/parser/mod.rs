pub mod block;
pub mod stmt;
pub mod expr;
pub mod parser;
pub mod lexer;
pub mod errors;

pub type ParseResult<T> = std::result::Result<T, crate::rjscript::parser::errors::ParseError>;