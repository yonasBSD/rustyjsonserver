
use std::collections::HashSet;

use crate::rjscript::{ast::{
    block::Block,
    expr::{Expr, ExprKind},
    stmt::StmtKind,
    visitor::{walk_block, walk_expr, Visit},
}, semantics::methods::builtin_names_set};
use crate::rjscript::preprocess::lints::error::LintError;
use crate::rjscript::preprocess::lints::util::{
    ident_name_from_callee, known_method_names_any,
    receiver_and_method_from_callee, collect_function_decls,
};

pub fn run(block: &Block) -> Vec<LintError> {
    let mut v = UnknownCalls::new(block);
    v.visit_block(block);
    v.errors
}

struct UnknownCalls {
    errors: Vec<LintError>,
    builtins: HashSet<&'static str>,
    known_methods: HashSet<&'static str>,
    user_funcs: HashSet<String>,
}

impl UnknownCalls {
    fn new(block: &Block) -> Self {
        let builtins = builtin_names_set();
        let known_methods = known_method_names_any();
        let user_funcs = collect_function_decls(block).into_keys().collect();
        Self { errors: Vec::new(), builtins, known_methods, user_funcs }
    }

    fn err_unknown_func(&mut self, pos: crate::rjscript::ast::position::Position, name: &str) {
        self.errors.push(LintError::new(pos, format!("Unknown function `{name}`")));
    }

    fn err_unknown_method(&mut self, pos: crate::rjscript::ast::position::Position, name: &str) {
        self.errors.push(LintError::new(pos, format!("Unknown method `{name}`")));
    }
}

impl Visit for UnknownCalls {
    fn visit_block(&mut self, b: &Block) {
        // Also collect nested function declarations
        for s in &b.stmts {
            if let StmtKind::FunctionDecl { ident, .. } = &s.kind {
                self.user_funcs.insert(ident.clone());
            }
        }
        walk_block(self, b);
    }

    fn visit_expr(&mut self, e: &Expr) {
        if let ExprKind::Call { callee, .. } = &e.kind {
            if let Some(name) = ident_name_from_callee(callee) {
                if !self.builtins.contains(name) && !self.user_funcs.contains(name) {
                    self.err_unknown_func(e.pos, name);
                }
            } else if let Some((_recv, method)) = receiver_and_method_from_callee(callee) {
                if !self.known_methods.contains(method) {
                    self.err_unknown_method(e.pos, method);
                }
            }
        }
        walk_expr(self, e);
    }
}
