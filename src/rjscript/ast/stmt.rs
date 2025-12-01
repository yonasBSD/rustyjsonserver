use crate::rjscript::{ast::{block::Block, expr::Expr, node::Located}, semantics::types::VarType};

#[derive(Debug, Clone)]
pub enum StmtKind  {
    /// `let name: ty = <expr>?;`
    Let {
        name: String,
        ty: VarType,
        init: Option<Expr>,
    },

    /// `return <expr>;`
    Return(Expr),

    /// `return <status>, <expr>;` (status, response)
    ReturnStatus { status: Expr, value: Expr },

    /// Standalone expression statement.
    ExprStmt(Expr),

    /// Function declaration.
    FunctionDecl {
        ident: String,
        params: Vec<(String, VarType)>,
        return_type: VarType,
        body: Block,
    },

    /// `if (cond) { then_block } else { else_block? }`
    IfElse {
        condition: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },

    /// switch (cond) { case, case2, default}
    Switch {
        condition: Expr,
        cases: Vec<(Expr, Block)>,
        default: Option<Block>,
    },

    /// `for (init; condition; increment) { body }`
    For {
        /// Optional initializer (let/assign or expr-stmt)
        init: Option<Box<Stmt>>,
        /// Loop condition; falsey (0 or false) breaks the loop
        condition: Expr,
        /// Optional increment expression run after each iteration
        increment: Option<Expr>,
        /// Body block
        body: Block,
    },

    Break,
    Continue
}

pub type Stmt = Located<StmtKind>;