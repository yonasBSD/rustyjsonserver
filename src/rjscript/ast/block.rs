use crate::rjscript::ast::{node::HasPos, position::Position, stmt::Stmt};

/// A block of statements delimited by braces.
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub pos: Position, // position of the opening brace or start of block
}

impl Block {
    #[inline]
    pub fn new(stmts: Vec<Stmt>, pos: Position) -> Self {
        Self { stmts, pos }
    }
}

impl HasPos for Block {
    #[inline]
    fn pos(&self) -> Position {
        self.pos
    }
}