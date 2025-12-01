use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub const UNKNOWN: Position = Position { line: 0, column: 0 };

    #[inline]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::UNKNOWN {
            write!(f, "?:?")
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}