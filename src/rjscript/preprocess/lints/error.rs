use std::cmp::Ordering;
use std::fmt;

use crate::rjscript::ast::position::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintError {
    pub pos: Position,
    pub message: String,
}

impl LintError {
    #[inline]
    pub fn new(pos: Position, message: impl Into<String>) -> Self {
        Self { pos, message: message.into() }
    }
}

impl Ord for LintError {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.pos.line, self.pos.column)
            .cmp(&(other.pos.line, other.pos.column))
            .then_with(|| self.message.cmp(&other.message))
    }
}
impl PartialOrd for LintError {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl fmt::Display for LintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} {}", self.pos.line, self.pos.column, self.message)
    }
}
