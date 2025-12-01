use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::rjscript::{
    ast::{
        block::Block,
        expr::{Expr, ExprKind, TemplatePart},
        node::HasPos,
        stmt::{Stmt, StmtKind},
        visitor::Visit,
    },
    preprocess::lints::{
        error::LintError,
        util::{ident_name_from_callee, Scope, ScopeRef},
    },
};

type ScopePtr = *const RefCell<Scope>;

#[derive(Clone, PartialEq, Eq, Hash)]
struct VarKey {
    owner: ScopePtr,
    name: String,
}

impl VarKey {
    fn new(owner: &ScopeRef, name: &str) -> Self {
        Self {
            owner: Rc::as_ptr(owner),
            name: name.to_string(),
        }
    }

    /// Build a VarKey for the nearest declaration of `name` in `cur`'s scope chain.
    fn varkey_from_decl(cur: &ScopeRef, name: &str) -> Option<Self> {
        Scope::find_decl_scope(cur, name).map(|owner| VarKey::new(&owner, name))
    }
}

#[derive(Clone, Default)]
struct AssignFacts {
    set: HashSet<VarKey>,
}

impl AssignFacts {
    fn has(&self, k: &VarKey) -> bool {
        self.set.contains(k)
    }

    fn mark(&mut self, k: VarKey) {
        self.set.insert(k);
    }

    /// Intersection for "must be assigned on all paths".
    fn intersect(&self, other: &AssignFacts) -> AssignFacts {
        let mut out = AssignFacts::default();
        for k in self.set.iter() {
            if other.set.contains(k) {
                out.set.insert(k.clone());
            }
        }
        out
    }
}

/// run the definite-assignment lint and return any errors.
pub fn run(block: &Block) -> Vec<LintError> {
    let mut pass = DefAssign::new();
    pass.visit_block(block);
    pass.errors
}

/// Tracks declaration scopes and "definitely-assigned" sets aligned to those scopes.
struct DefAssign {
    errors: Vec<LintError>,
    facts: AssignFacts,
    /// Lexical declaration stack (names declared in each scope).
    pub cur_scope: ScopeRef,
}

impl DefAssign {
    fn new() -> Self {
        Self {
            cur_scope: Scope::new_root(),
            facts: AssignFacts::default(),
            errors: Vec::new(),
        }
    }

    fn err(&mut self, pos: crate::rjscript::ast::position::Position, msg: impl Into<String>) {
        self.errors.push(LintError::new(pos, msg.into()));
    }

    fn with_scope_and_facts<F>(&mut self, scope: ScopeRef, facts: &mut AssignFacts, mut f: F)
    where
        F: FnMut(&mut Self),
    {
        let prev_scope = self.cur_scope.clone();
        let prev_facts = std::mem::replace(&mut self.facts, facts.clone());
        self.cur_scope = scope;

        f(self);

        *facts = self.facts.clone();
        self.cur_scope = prev_scope;
        self.facts = prev_facts;
    }

    /// Register a variable *use* (read).
    fn use_var(&mut self, name: &str, at: &Expr) {
        // Must be declared (lexically) somewhere up the chain.
        if !Scope::has_var_in_chain(&self.cur_scope, name) {
            self.err(at.pos(), format!("`{}` used before declaration", name));
            return;
        }
        // Must be assigned for the nearest declaration.
        if let Some(key) = VarKey::varkey_from_decl(&self.cur_scope, name) {
            if !self.facts.has(&key) {
                self.err(
                    at.pos(),
                    format!("`{}` used before assignment", name),
                );
            }
        }
    }
}

impl Visit for DefAssign {
    fn visit_stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let { name, init, .. } => {
                Scope::declare_var(&self.cur_scope, name);
                if let Some(rhs) = init {
                    self.visit_expr(rhs);
                    self.facts.mark(VarKey::new(&self.cur_scope, name));
                }
            }

            StmtKind::ExprStmt(e) => {
                self.visit_expr(e);
            }

            StmtKind::Return(e) => {
                self.visit_expr(e);
            }

            StmtKind::ReturnStatus { status, value } => {
                self.visit_expr(status);
                self.visit_expr(value);
            }

            StmtKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                // Condition: reads (and any side-effect assignments in condition) happen before branches.
                self.visit_expr(condition);

                let incoming = self.facts.clone();

                // THEN branch
                let then_scope = Scope::push_child(&self.cur_scope);
                let mut then_facts = incoming.clone();
                self.with_scope_and_facts(then_scope, &mut then_facts, |this| {
                    this.visit_block(then_block);
                });

                // ELSE branch
                let (has_else, else_facts) = if let Some(else_blk) = else_block {
                    let else_scope = Scope::push_child(&self.cur_scope);
                    let mut f = incoming.clone();
                    self.with_scope_and_facts(else_scope, &mut f, |this| {
                        this.visit_block(else_blk);
                    });
                    (true, f)
                } else {
                    (false, incoming.clone())
                };

                // Merge
                self.facts = if has_else {
                    // both branches exist -> intersection
                    then_facts.intersect(&else_facts)
                } else {
                    incoming
                };
            }

            // for (init; condition; increment) { body }
            StmtKind::For {
                init,
                condition,
                increment,
                body,
            } => {
                let loop_scope = Scope::push_child(&self.cur_scope);
                let mut facts_after_init_cond = self.facts.clone();

                // Run init in loop scope – assignments here do flow out
                self.with_scope_and_facts(loop_scope.clone(), &mut facts_after_init_cond, |this| {
                    if let Some(init_stmt) = init {
                        this.visit_stmt(init_stmt);
                    }
                    this.visit_expr(condition);
                });

                // Body (does not flow out)
                let mut body_facts = facts_after_init_cond.clone();
                self.with_scope_and_facts(loop_scope.clone(), &mut body_facts, |this| {
                    this.visit_block(body);
                });

                // Increment (does not flow out)
                if let Some(inc) = increment {
                    let mut inc_facts = facts_after_init_cond.clone();
                    self.with_scope_and_facts(loop_scope.clone(), &mut inc_facts, |this| {
                        this.visit_expr(inc);
                    });
                }

                // After loop: only init+condition effects are definite.
                self.facts = facts_after_init_cond;
            }

            // switch (condition) { case e: ...; default: ...; }
            StmtKind::Switch {
                condition,
                cases,
                default,
            } => {
                // Condition is always evaluated
                self.visit_expr(condition);

                let incoming = self.facts.clone();
                let mut paths: Vec<AssignFacts> = Vec::with_capacity(cases.len() + 1);

                // Cases
                for (case_expr, case_block) in cases {
                    // Evaluate case expr (reads only)
                    self.visit_expr(case_expr);

                    // Case block in its own scope, forked facts from incoming
                    let case_scope = Scope::push_child(&self.cur_scope);
                    let mut f = incoming.clone();
                    self.with_scope_and_facts(case_scope, &mut f, |this| {
                        this.visit_block(case_block);
                    });
                    paths.push(f);
                }

                // Default
                if let Some(def_block) = default {
                    let def_scope = Scope::push_child(&self.cur_scope);
                    let mut f = incoming.clone();
                    self.with_scope_and_facts(def_scope, &mut f, |this| {
                        this.visit_block(def_block);
                    });
                    paths.push(f);
                } else {
                    // No default: a path where no case matched
                    paths.push(incoming.clone());
                }

                // Intersect across all paths (single consumption of `paths`)
                let mut iter = paths.into_iter();
                if let Some(first) = iter.next() {
                    let acc = iter.fold(first, |acc, f| acc.intersect(&f));
                    self.facts = acc;
                } else {
                    self.facts = incoming;
                }
            }

            // function f(params) { body }
            // - Function name is in a separate namespace; outer facts unaffected.
            // - Parameters are declared+assigned at body entry.
            StmtKind::FunctionDecl {
                ident: _,
                params,
                body,
                ..
            } => {
                // Build function-body scope
                let body_scope = Scope::push_child(&self.cur_scope);

                // Start with fresh facts for the function body
                // (function-local assignment facts do not leak out)
                let mut inner_facts = AssignFacts::default();

                // Params are vars and considered assigned on entry:
                // declare them in the body scope and mark as assigned in inner_facts
                for (pname, _pty) in params {
                    Scope::declare_var(&body_scope, pname);
                    if let Some(k) = VarKey::varkey_from_decl(&body_scope, pname) {
                        inner_facts.mark(k);
                    }
                }

                // Run the body inside that scope with the pre-seeded facts
                self.with_scope_and_facts(body_scope, &mut inner_facts, |this| {
                    this.visit_block(body);
                });
            }

            StmtKind::Break | StmtKind::Continue => {
                // Nothing for DA here; control-flow merging handled at branch level.
            }
        }
    }

    fn visit_expr(&mut self, e: &Expr) {
        match &e.kind {
            // Variable read
            ExprKind::Ident(name) => {
                self.use_var(name, e);
            }

            // Binary ops: visit both sides
            ExprKind::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }

            // Member access: object is read
            ExprKind::Member { object, .. } => {
                self.visit_expr(object);
            }

            // Indexing: object and index are read
            ExprKind::Index { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }

            // Call: skip bare identifier callee (functions live in a separate namespace),
            // but still visit non-ident callees (e.g., obj.method) and all arguments.
            ExprKind::Call { callee, args } => {
                let skip_callee_ident = ident_name_from_callee(callee).is_some();
                if !skip_callee_ident {
                    self.visit_expr(callee);
                }
                for a in args {
                    self.visit_expr(a);
                }
            }

            // Array/Object/Template: recurse
            ExprKind::Array(items) => {
                for it in items {
                    self.visit_expr(it);
                }
            }
            ExprKind::ObjectLiteral { fields } => {
                for (_, ex) in fields {
                    self.visit_expr(ex);
                }
            }
            ExprKind::Template(parts) => {
                for p in parts {
                    if let TemplatePart::Expr(ex) = p {
                        self.visit_expr(ex);
                    }
                }
            }

            // Literal / type / request field: no reads
            ExprKind::Literal(_) | ExprKind::TypeLiteral(_) | ExprKind::RequestField(_) => {}

            // Assignments inside expressions:
            // - For `x = <value>`: first visit RHS (reads), then mark `x` assigned.
            // - For member/index assignment: visit subexpressions but do not mark a var as assigned.
            ExprKind::AssignVar { name, value } => {
                self.visit_expr(value);
                if let Some(key) = VarKey::varkey_from_decl(&self.cur_scope, name) {
                    self.facts.mark(key);
                } else {
                    // assigning an undeclared var should be caught by other lints;
                }
            }
            ExprKind::AssignMember { object, value, .. } => {
                self.visit_expr(object);
                self.visit_expr(value);
            }
            ExprKind::AssignIndex {
                object,
                index,
                value,
            } => {
                self.visit_expr(object);
                self.visit_expr(index);
                self.visit_expr(value);
            }
        }
    }
}
