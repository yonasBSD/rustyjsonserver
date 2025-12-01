use crate::rjscript::ast::expr::{Expr, ExprKind};
use crate::rjscript::ast::position::Position;
use crate::rjscript::ast::visitor::walk_expr;
use crate::rjscript::preprocess::lints::error::LintError;
use crate::rjscript::{
    ast::{
        block::Block,
        stmt::{Stmt, StmtKind},
        visitor::Visit,
    },
    preprocess::lints::util::{Scope, ScopeRef},
};

pub fn run(block: &Block) -> Vec<LintError> {
    let mut v = Declarations::new();
    v.visit_block(block);
    v.errors
}

struct Declarations {
    errors: Vec<LintError>,
    cur_scope: ScopeRef,
}

impl Declarations {
    pub fn new() -> Self {
        Declarations {
            errors: Vec::new(),
            cur_scope: Scope::new_root(),
        }
    }

    fn push_block(&mut self) {
        let child = Scope::push_child(&self.cur_scope);
        self.cur_scope = child;
    }

    fn pop_block(&mut self) {
        let parent =
            Scope::parent(&self.cur_scope).expect("Declarations visitor: tried to pop past root");
        self.cur_scope = parent;
    }

    /// Run a closure in a freshly pushed child scope, then pop.
    fn in_child_scope<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Self),
    {
        self.push_block();
        f(self);
        self.pop_block();
    }

    /// Run a closure in a specific scope (e.g., prepared function body scope).
    fn with_scope<F>(&mut self, scope: ScopeRef, mut f: F)
    where
        F: FnMut(&mut Self),
    {
        let prev = self.cur_scope.clone();
        self.cur_scope = scope;
        f(self);
        self.cur_scope = prev;
    }

    fn err(&mut self, pos: Position, message: String) {
        self.errors.push(LintError::new(pos, message));
    }
}

impl Visit for Declarations {
    fn visit_stmt(&mut self, s: &Stmt) {
        match &s.kind {
            // Variables: flag only if an *outer* variable with same name exists.
            StmtKind::Let { name, .. } => {
                if Scope::has_var_in_chain(&self.cur_scope, name) {
                    self.err(s.pos, format!("`{}` already declared", name));
                }
                Scope::declare_var(&self.cur_scope, name);
            }

            // Functions: separate namespace; flag only if an *outer* function exists.
            // Function name may equal a variable name (allowed).
            StmtKind::FunctionDecl {
                ident,
                params,
                body,
                ..
            } => {
                if Scope::has_fn_in_chain(&self.cur_scope, ident) {
                    self.err(s.pos, format!("function `{}` already declared", ident));
                }
                // Declare the function in the *current* function namespace.
                Scope::declare_fn(&self.cur_scope, ident);

                // Create a dedicated scope for the function body; insert params as variables.
                let body_scope = Scope::push_child(&self.cur_scope);
                for (pname, _pty) in params {
                    Scope::declare_var(&body_scope, pname);
                }

                // Visit the body within that scope (no extra push/pop).
                self.with_scope(body_scope, |this| {
                    this.visit_block(body);
                });
            }

            StmtKind::IfElse {
                then_block,
                else_block,
                ..
            } => {
                self.in_child_scope(|this| {
                    this.visit_block(then_block);
                });
                if let Some(else_b) = else_block {
                    self.in_child_scope(|this| {
                        this.visit_block(else_b);
                    });
                }
            }

            StmtKind::For { init, body, .. } => {
                // Loop header+body share one child scope for this lint
                self.in_child_scope(|this| {
                    if let Some(init_stmt) = init.as_deref() {
                        this.visit_stmt(init_stmt);
                    }
                    this.visit_block(body);
                });
            }

            StmtKind::Switch { cases, default, .. } => {
                for (_cexpr, cblock) in cases {
                    self.in_child_scope(|this| {
                        this.visit_block(cblock);
                    });
                }
                if let Some(def_block) = default {
                    self.in_child_scope(|this| {
                        this.visit_block(def_block);
                    });
                }
            }

            StmtKind::ExprStmt(e) => {
                self.visit_expr(e);
            }

            // The rest do not declare names.
            StmtKind::Return(_)
            | StmtKind::ReturnStatus { .. }
            | StmtKind::Break
            | StmtKind::Continue => {}
        }
    }

    fn visit_expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::AssignVar { name, .. } => {
                if !Scope::has_var_in_chain(&self.cur_scope, name) {
                    self.err(e.pos, format!("Variable `{}` is not declared", name));
                }
            },
            ExprKind::TypeLiteral(_)
            | ExprKind::Literal(_)
            | ExprKind::Template(_)
            | ExprKind::Ident(_)
            | ExprKind::ObjectLiteral { .. }
            | ExprKind::Array(_)
            | ExprKind::AssignMember {
                ..
            }
            | ExprKind::AssignIndex {
                ..
            }
            | ExprKind::Index { .. }
            | ExprKind::RequestField(_)
            | ExprKind::BinaryOp {.. }
            | ExprKind::Member { .. }
            | ExprKind::Call { .. } => {}
        }
        walk_expr(self, e)
    }
}
