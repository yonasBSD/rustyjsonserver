use crate::rjscript::ast::{
    block::Block,
    stmt::{Stmt, StmtKind},
    visitor::{walk_stmt, Visit},
};
use crate::rjscript::preprocess::lints::error::LintError;

pub fn run(block: &Block) -> Vec<LintError> {
    let mut v = MustReturn { errors: Vec::new() };
    v.check_top_level(block);
    v.visit_block(block);
    v.errors
}

pub fn block_returns(b: &Block) -> bool {
    for s in &b.stmts {
        match &s.kind {
            StmtKind::Return(_) | StmtKind::ReturnStatus { .. } => {
                return true;
            }
            StmtKind::IfElse {
                condition: _,
                then_block,
                else_block,
            } => {
                let then_ret = block_returns(then_block);
                let else_ret = else_block
                    .as_ref()
                    .map(|b| block_returns(b))
                    .unwrap_or(false);
                if then_ret && else_ret {
                    return true;
                }
            }
            StmtKind::Switch { cases, default, .. } => {
                // require: every case and default returns
                let all_cases = cases.iter().all(|(_, b)| block_returns(b));
                let has_default = default.as_ref().map(|b| block_returns(b)).unwrap_or(false);
                if all_cases && has_default {
                    return true;
                }
            }
            StmtKind::For { body, .. } => {
                if block_returns(body) {
                    return true;
                }
            }
            StmtKind::FunctionDecl { .. }
            | StmtKind::Break
            | StmtKind::ExprStmt(_)
            | StmtKind::Let { .. }
            | StmtKind::Continue => {}
        }
    }
    return false;
}

struct MustReturn {
    errors: Vec<LintError>,
}

impl MustReturn {
    fn err(&mut self, pos: crate::rjscript::ast::position::Position, name: &str) {
        self.errors.push(LintError::new(
            pos,
            format!("Function `{name}` does not return on all paths"),
        ));
    }

    fn check_top_level(&mut self, b: &Block) {
        if !block_returns(b) {
            self.errors
                .push(LintError::new(b.pos, "Script does not return on all paths"));
        }
    }
}

impl Visit for MustReturn {
    fn visit_stmt(&mut self, s: &Stmt) {
        if let StmtKind::FunctionDecl { ident, body, .. } = &s.kind {
            if !block_returns(body) {
                self.err(s.pos, ident);
            }
        }
        walk_stmt(self, s);
    }
}
