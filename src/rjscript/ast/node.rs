use super::position::Position;

/// Minimal position-carrying wrapper for any AST node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<T> {
    pub kind: T,
    pub pos: Position,
}

impl<T> Located<T> {
    #[inline]
    pub fn new(kind: T, pos: Position) -> Self {
        Self { kind, pos }
    }

    #[inline]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Located<U> {
        Located {
            kind: f(self.kind),
            pos: self.pos,
        }
    }

    #[inline]
    pub fn as_ref(&self) -> Located<&T> {
        Located {
            kind: &self.kind,
            pos: self.pos,
        }
    }

    #[inline]
    pub fn as_mut(&mut self) -> Located<&mut T> {
        let pos = self.pos;
        Located {
            kind: &mut self.kind,
            pos,
        }
    }
}

pub trait HasPos {
    fn pos(&self) -> Position;
}

impl<T> HasPos for Located<T> {
    #[inline]
    fn pos(&self) -> Position {
        self.pos
    }
}
