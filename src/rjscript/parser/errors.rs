use std::fmt;

use crate::rjscript::ast::position::Position;

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedEOF(Position),
    UnexpectedChar(char, Position),
    ExpectedIdentifier(String, Position),
    ExpectedNumber(Position),
    MissingEqualsAfterLet(Position),
    UnexpectedValueAfterReq(String, Position),
    MissingDotAfterReq(Position),
    MissingDotAfterBody(Position),
    MissingDotAfterParams(Position),
    MissingDotAfterQuery(Position),
    MissingClosingParen(Position),
    ExtraCharacters(Position),
    InvalidEscape(char, Position),
    UnterminatedString(Position),
    InvalidAssignmentTarget(Position),
    ExpectedExpression(Position),
    General(String, Position),
}

impl ParseError {
    pub fn pos(&self) -> Position {
        let pos: Position = match *self {
            ParseError::UnexpectedEOF(position) => position,
            ParseError::UnexpectedChar(_, position) => position,
            ParseError::ExpectedIdentifier(_, position) => position,
            ParseError::ExpectedNumber(position) => position,
            ParseError::MissingEqualsAfterLet(position) => position,
            ParseError::UnexpectedValueAfterReq(_, position) => position,
            ParseError::MissingDotAfterReq(position) => position,
            ParseError::MissingDotAfterBody(position) => position,
            ParseError::MissingDotAfterParams(position) => position,
            ParseError::MissingDotAfterQuery(position) => position,
            ParseError::MissingClosingParen(position) => position,
            ParseError::ExtraCharacters(position) => position,
            ParseError::InvalidEscape(_, position) => position,
            ParseError::UnterminatedString(position) => position,
            ParseError::InvalidAssignmentTarget(position) => position,
            ParseError::ExpectedExpression(position) => position,
            ParseError::General(_, position) => position,
        };

        pos
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEOF(at) => {
                write!(f, "Unexpected end of input, at {}:{}", at.line, at.column)
            }
            ParseError::UnexpectedChar(ch, at) => {
                write!(f, "Unexpected char '{}', at {}:{}", ch, at.line, at.column)
            }
            ParseError::ExpectedIdentifier(found, at) => {
                write!(
                    f,
                    "Expected identifier, but found '{}', at {}:{}",
                    found, at.line, at.column
                )
            }
            ParseError::UnexpectedValueAfterReq(ch, at) => {
                write!(
                    f,
                    "Unexpected identifier found after req, found '{}', at {}:{}",
                    ch, at.line, at.column
                )
            }
            ParseError::ExpectedNumber(at) => {
                write!(f, "Expected number, at {}:{}", at.line, at.column)
            }
            ParseError::MissingEqualsAfterLet(at) => {
                write!(
                    f,
                    "Expected '=' after 'let' <identifier>, at {}:{}",
                    at.line, at.column
                )
            }
            ParseError::MissingDotAfterReq(at) => {
                write!(f, "Expected '.' after 'req', at {}:{}", at.line, at.column)
            }
            ParseError::MissingDotAfterBody(at) => {
                write!(f, "Expected '.' after 'body', at {}:{}", at.line, at.column)
            }
            ParseError::MissingDotAfterParams(at) => {
                write!(
                    f,
                    "Expected '.' after 'params', at {}:{}",
                    at.line, at.column
                )
            }
            ParseError::MissingDotAfterQuery(at) => {
                write!(
                    f,
                    "Expected '.' after 'query', at {}:{}",
                    at.line, at.column
                )
            }
            ParseError::MissingClosingParen(at) => {
                write!(
                    f,
                    "Missing closing parenthesis, at {}:{}",
                    at.line, at.column
                )
            }
            ParseError::ExtraCharacters(at) => {
                write!(
                    f,
                    "Extra characters after script, at {}:{}",
                    at.line, at.column
                )
            }
            ParseError::InvalidEscape(ch, at) => {
                write!(
                    f,
                    "Invalid escape character, found '{}', at {}:{}",
                    ch, at.line, at.column
                )
            }
            ParseError::UnterminatedString(at) => {
                write!(
                    f,
                    "Unterminated string, expected closing quote at, {}:{}",
                    at.line, at.column
                )
            }
            ParseError::InvalidAssignmentTarget(at) => write!(
                f,
                "Invalid assignment type detected, at {}:{}",
                at.line, at.column
            ),
            ParseError::ExpectedExpression(at) => {
                write!(f, "Expected expression, at {}:{}", at.line, at.column)
            }
            ParseError::General(msg, at) => write!(f, "{}, at {}:{}", msg, at.line, at.column),
        }
    }
}

impl std::error::Error for ParseError {}
