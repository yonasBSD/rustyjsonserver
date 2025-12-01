use crate::rjscript::{
    ast::{binop::BinOp, literal::Literal, node::Located, request::RequestFieldType},
    semantics::types::VarType,
};

/// A position-carrying expression.
pub type Expr = Located<ExprKind>;

impl Expr {
    pub fn is_request_derived(&self) -> bool {
        match &self.kind {
            ExprKind::RequestField(..) => true,
            ExprKind::Member { object, .. }
            | ExprKind::Index { object, .. }
            | ExprKind::Call { callee: object, .. } => object.is_request_derived(),
            ExprKind::Array(items) => items.iter().any(|e| e.is_request_derived()),
            ExprKind::BinaryOp { left, right, .. } => {
                left.is_request_derived() || right.is_request_derived()
            }
            ExprKind::AssignVar { value, .. }
            | ExprKind::AssignMember { value, .. }
            | ExprKind::AssignIndex { value, .. } => value.is_request_derived(),
            _ => false,
        }
    }

    /// Returns the left-most identifier name of an expression chain (e.g., `req.body.x[0]().y` -> "req")
    pub fn root_ident<'a>(mut e: &'a Expr) -> Option<&'a str> {
        loop {
            match &e.kind {
                ExprKind::Ident(name) => return Some(name.as_str()),
                ExprKind::Member { object, .. } => e = object,
                ExprKind::Index { object, .. } => e = object,
                ExprKind::Call { callee, .. } => e = callee,
                _ => return None,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    TypeLiteral(VarType),
    Literal(Literal),
    Template(Vec<TemplatePart>),

    /// Local variable, e.g. `x`.
    Ident(String),

    /// Object literal: `{ key: expr, ... }`
    ObjectLiteral {
        fields: Vec<(String, Expr)>,
    },

    /// Array literal: `[expr, ...]`
    Array(Vec<Expr>),

    /// `x = expr`
    AssignVar {
        name: String,
        value: Box<Expr>,
    },

    /// `obj.prop = expr`
    AssignMember {
        object: Box<Expr>,
        property: String,
        value: Box<Expr>,
    },

    /// `obj[idx] = expr`
    AssignIndex {
        object: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
    },

    /// `obj[idx]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    /// `req.body.x`, `req.params.id`, etc.
    RequestField(RequestFieldType),

    /// Binary operator: `left op right`
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// `obj.prop`
    Member {
        object: Box<Expr>,
        property: String,
    },

    /// Function or method call: `callee(args...)`
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    Text(String),
    Expr(Expr),
}
