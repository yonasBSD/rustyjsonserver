use crate::rjscript::ast::{
    block::Block,
    expr::{Expr, ExprKind},
    visitor::{Visit, walk_block, walk_expr},
};
use crate::rjscript::preprocess::lints::error::LintError;
use crate::rjscript::preprocess::lints::util::{receiver_and_method_from_callee, is_mutating_method_any};

pub fn run(block: &Block) -> Vec<LintError> {
    let mut v = ReqImmut::default();
    v.visit_block(block);
    v.errors
}

#[derive(Default)]
struct ReqImmut {
    errors: Vec<LintError>,
}

impl ReqImmut {
    fn err(&mut self, pos: crate::rjscript::ast::position::Position) {
        self.errors.push(LintError::new(pos, "You cannot mutate `req` or its fields"));
    }
}

impl Visit for ReqImmut {
    fn visit_block(&mut self, b: &Block) {
        walk_block(self, b);
    }

    fn visit_expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::AssignMember { object, .. } | ExprKind::AssignIndex { object, .. } => {
                if object.is_request_derived() { self.err(e.pos); }
            }
            ExprKind::AssignVar { .. } => { /* assigning a local is fine */ }
            ExprKind::Call { callee, .. } => {
                if let Some((recv, method)) = receiver_and_method_from_callee(callee) {
                    if recv.is_request_derived() && is_mutating_method_any(method) {
                        self.err(e.pos);
                    }
                }
            }
            _ => {}
        }
        walk_expr(self, e);
    }
}
