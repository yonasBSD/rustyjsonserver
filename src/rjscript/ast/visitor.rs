use crate::rjscript::ast::node::HasPos;

use super::{
    block::Block,
    expr::{Expr, ExprKind, TemplatePart},
    node::Located,
    stmt::{Stmt, StmtKind},
};

pub trait Visit {
    fn visit_block(&mut self, b: &Block) {
        walk_block(self, b)
    }
    fn visit_stmt(&mut self, s: &Stmt) {
        walk_stmt(self, s)
    }
    fn visit_expr(&mut self, e: &Expr) {
        walk_expr(self, e)
    }
}

pub fn walk_block<V: Visit + ?Sized>(v: &mut V, b: &Block) {
    for s in &b.stmts {
        v.visit_stmt(s);
    }
}

pub fn walk_stmt<V: Visit + ?Sized>(v: &mut V, s: &Stmt) {
    match &s.kind {
        StmtKind::Let { init, .. } => {
            if let Some(e) = init {
                v.visit_expr(e);
            }
        }
        StmtKind::Return(e) => v.visit_expr(e),
        StmtKind::ReturnStatus { status, value } => {
            v.visit_expr(status);
            v.visit_expr(value);
        }
        StmtKind::ExprStmt(e) => v.visit_expr(e),
        StmtKind::FunctionDecl { body, .. } => v.visit_block(body),
        StmtKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            v.visit_expr(condition);
            v.visit_block(then_block);
            if let Some(b) = else_block {
                v.visit_block(b);
            }
        }
        StmtKind::Switch {
            condition,
            cases,
            default,
        } => {
            v.visit_expr(condition);
            for (e, b) in cases.iter() {
                v.visit_expr(e);
                v.visit_block(b);
            }
            if let Some(b) = default {
                v.visit_block(b);
            }
        }
        StmtKind::For {
            init,
            condition,
            increment,
            body,
        } => {
            if let Some(s0) = init.as_deref() {
                v.visit_stmt(s0);
            }
            v.visit_expr(condition);
            if let Some(inc) = increment {
                v.visit_expr(inc);
            }
            v.visit_block(body);
        }
        StmtKind::Break | StmtKind::Continue => {}
    }
}

pub fn walk_expr<V: Visit + ?Sized>(v: &mut V, e: &Expr) {
    match &e.kind {
        ExprKind::TypeLiteral(_)
        | ExprKind::Literal(_)
        | ExprKind::Ident(_)
        | ExprKind::RequestField(_) => {}
        ExprKind::Template(parts) => {
            for p in parts {
                match p {
                    TemplatePart::Text(_) => {}
                    TemplatePart::Expr(ex) => v.visit_expr(ex),
                }
            }
        }
        ExprKind::ObjectLiteral { fields } => {
            for (_, ex) in fields {
                v.visit_expr(ex);
            }
        }
        ExprKind::Array(items) => {
            for it in items {
                v.visit_expr(it);
            }
        }
        ExprKind::AssignVar { value, .. } => v.visit_expr(value),
        ExprKind::AssignMember { object, value, .. } => {
            v.visit_expr(object);
            v.visit_expr(value);
        }
        ExprKind::AssignIndex {
            object,
            index,
            value,
        } => {
            v.visit_expr(object);
            v.visit_expr(index);
            v.visit_expr(value);
        }
        ExprKind::Index { object, index } => {
            v.visit_expr(object);
            v.visit_expr(index);
        }
        ExprKind::Member { object, .. } => v.visit_expr(object),
        ExprKind::BinaryOp { left, right, .. } => {
            v.visit_expr(left);
            v.visit_expr(right);
        }
        ExprKind::Call { callee, args } => {
            v.visit_expr(callee);
            for a in args {
                v.visit_expr(a);
            }
        }
    }
}

pub trait VisitMut {
    fn visit_block_mut(&mut self, b: &mut Block) {
        walk_block_mut(self, b)
    }
    fn visit_stmt_mut(&mut self, s: &mut Stmt) {
        walk_stmt_mut(self, s)
    }
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        walk_expr_mut(self, e)
    }
    fn visit_template_part_mut(&mut self, p: &mut TemplatePart) {
        match p {
            TemplatePart::Text(_) => {}
            TemplatePart::Expr(e) => self.visit_expr_mut(e),
        }
    }
}

pub fn walk_block_mut<V: VisitMut + ?Sized>(v: &mut V, b: &mut Block) {
    for s in &mut b.stmts {
        v.visit_stmt_mut(s);
    }
}

pub fn walk_stmt_mut<V: VisitMut + ?Sized>(v: &mut V, s: &mut Stmt) {
    match &mut s.kind {
        StmtKind::Let { init, .. } => {
            if let Some(e) = init {
                v.visit_expr_mut(e);
            }
        }
        StmtKind::Return(e) => v.visit_expr_mut(e),
        StmtKind::ReturnStatus { status, value } => {
            v.visit_expr_mut(status);
            v.visit_expr_mut(value);
        }
        StmtKind::ExprStmt(e) => v.visit_expr_mut(e),
        StmtKind::FunctionDecl { body, .. } => v.visit_block_mut(body),
        StmtKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            v.visit_expr_mut(condition);
            v.visit_block_mut(then_block);
            if let Some(b) = else_block {
                v.visit_block_mut(b);
            }
        }
        StmtKind::Switch {
            condition,
            cases,
            default,
        } => {
            v.visit_expr_mut(condition);
            for (e, b) in cases.iter_mut() {
                v.visit_expr_mut(e);
                v.visit_block_mut(b);
            }
            if let Some(b) = default {
                v.visit_block_mut(b);
            }
        }
        StmtKind::For {
            init,
            condition,
            increment,
            body,
        } => {
            if let Some(s0) = init.as_deref_mut() {
                v.visit_stmt_mut(s0);
            }
            v.visit_expr_mut(condition);
            if let Some(inc) = increment {
                v.visit_expr_mut(inc);
            }
            v.visit_block_mut(body);
        }
        StmtKind::Break | StmtKind::Continue => {}
    }
}

pub fn walk_expr_mut<V: VisitMut + ?Sized>(v: &mut V, e: &mut Expr) {
    match &mut e.kind {
        ExprKind::TypeLiteral(_)
        | ExprKind::Literal(_)
        | ExprKind::Ident(_)
        | ExprKind::RequestField(_) => {}
        ExprKind::Template(parts) => {
            for p in parts {
                v.visit_template_part_mut(p);
            }
        }
        ExprKind::ObjectLiteral { fields } => {
            for (_, ex) in fields {
                v.visit_expr_mut(ex);
            }
        }
        ExprKind::Array(items) => {
            for it in items {
                v.visit_expr_mut(it);
            }
        }
        ExprKind::AssignVar { value, .. } => v.visit_expr_mut(value),
        ExprKind::AssignMember { object, value, .. } => {
            v.visit_expr_mut(object);
            v.visit_expr_mut(value);
        }
        ExprKind::AssignIndex {
            object,
            index,
            value,
        } => {
            v.visit_expr_mut(object);
            v.visit_expr_mut(index);
            v.visit_expr_mut(value);
        }
        ExprKind::Index { object, index } => {
            v.visit_expr_mut(object);
            v.visit_expr_mut(index);
        }
        ExprKind::Member { object, .. } => v.visit_expr_mut(object),
        ExprKind::BinaryOp { left, right, .. } => {
            v.visit_expr_mut(left);
            v.visit_expr_mut(right);
        }
        ExprKind::Call { callee, args } => {
            v.visit_expr_mut(callee);
            for a in args {
                v.visit_expr_mut(a);
            }
        }
    }
}

//
// ----------------------------- Fold (return-new) ----------------------------
//

pub trait Fold {
    fn fold_block(&mut self, b: Block) -> Block {
        fold_block(self, b)
    }
    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        fold_stmt(self, s)
    }
    fn fold_expr(&mut self, e: Expr) -> Expr {
        fold_expr(self, e)
    }
    fn fold_template_part(&mut self, p: TemplatePart) -> TemplatePart {
        match p {
            TemplatePart::Text(t) => TemplatePart::Text(t),
            TemplatePart::Expr(e) => TemplatePart::Expr(self.fold_expr(e)),
        }
    }
}

pub fn fold_block<F: Fold + ?Sized>(f: &mut F, mut b: Block) -> Block {
    let stmts = b.stmts.into_iter().map(|s| f.fold_stmt(s)).collect();
    b.stmts = stmts;
    b
}

pub fn fold_stmt<F: Fold + ?Sized>(f: &mut F, s: Stmt) -> Stmt {
    let pos = s.pos();
    match s.kind {
        StmtKind::Let { name, ty, init } => {
            let init = init.map(|e| f.fold_expr(e));
            Located::new(StmtKind::Let { name, ty, init }, pos)
        }
        StmtKind::Return(e) => Located::new(StmtKind::Return(f.fold_expr(e)), pos),
        StmtKind::ReturnStatus { status, value } => Located::new(
            StmtKind::ReturnStatus {
                status: f.fold_expr(status),
                value: f.fold_expr(value),
            },
            pos,
        ),
        StmtKind::ExprStmt(e) => Located::new(StmtKind::ExprStmt(f.fold_expr(e)), pos),
        StmtKind::FunctionDecl {
            ident,
            params,
            return_type,
            body,
        } => {
            let body = f.fold_block(body);
            Located::new(
                StmtKind::FunctionDecl {
                    ident,
                    params,
                    return_type,
                    body,
                },
                pos,
            )
        }
        StmtKind::IfElse {
            condition,
            then_block,
            else_block,
        } => {
            let condition = f.fold_expr(condition);
            let then_block = f.fold_block(then_block);
            let else_block = else_block.map(|b| f.fold_block(b));
            Located::new(
                StmtKind::IfElse {
                    condition,
                    then_block,
                    else_block,
                },
                pos,
            )
        }
        StmtKind::Switch {
            condition,
            cases,
            default,
        } => {
            let condition = f.fold_expr(condition);
            let cases = cases
                .into_iter()
                .map(|(e, b)| (f.fold_expr(e), f.fold_block(b)))
                .collect();
            let default = default.map(|b| f.fold_block(b));
            Located::new(
                StmtKind::Switch {
                    condition,
                    cases,
                    default,
                },
                pos,
            )
        }
        StmtKind::For {
            init,
            condition,
            increment,
            body,
        } => {
            let init = init.map(|s| Box::new(f.fold_stmt(*s)));
            let condition = f.fold_expr(condition);
            let increment = increment.map(|e| f.fold_expr(e));
            let body = f.fold_block(body);
            Located::new(
                StmtKind::For {
                    init,
                    condition,
                    increment,
                    body,
                },
                pos,
            )
        }
        StmtKind::Break => Located::new(StmtKind::Break, pos),
        StmtKind::Continue => Located::new(StmtKind::Continue, pos),
    }
}

pub fn fold_expr<F: Fold + ?Sized>(f: &mut F, e: Expr) -> Expr {
    let pos = e.pos();
    match e.kind {
        ExprKind::TypeLiteral(t) => Located::new(ExprKind::TypeLiteral(t), pos),
        ExprKind::Literal(v) => Located::new(ExprKind::Literal(v), pos),
        ExprKind::Ident(n) => Located::new(ExprKind::Ident(n), pos),
        ExprKind::RequestField(r) => Located::new(ExprKind::RequestField(r), pos),
        ExprKind::Template(parts) => {
            let parts = parts.into_iter().map(|p| f.fold_template_part(p)).collect();
            Located::new(ExprKind::Template(parts), pos)
        }
        ExprKind::ObjectLiteral { fields } => {
            let fields = fields
                .into_iter()
                .map(|(k, ex)| (k, f.fold_expr(ex)))
                .collect();
            Located::new(ExprKind::ObjectLiteral { fields }, pos)
        }
        ExprKind::Array(items) => {
            let items = items.into_iter().map(|it| f.fold_expr(it)).collect();
            Located::new(ExprKind::Array(items), pos)
        }
        ExprKind::AssignVar { name, value } => {
            let value = Box::new(f.fold_expr(*value));
            Located::new(ExprKind::AssignVar { name, value }, pos)
        }
        ExprKind::AssignMember {
            object,
            property,
            value,
        } => {
            let object = Box::new(f.fold_expr(*object));
            let value = Box::new(f.fold_expr(*value));
            Located::new(
                ExprKind::AssignMember {
                    object,
                    property,
                    value,
                },
                pos,
            )
        }
        ExprKind::AssignIndex {
            object,
            index,
            value,
        } => {
            let object = Box::new(f.fold_expr(*object));
            let index = Box::new(f.fold_expr(*index));
            let value = Box::new(f.fold_expr(*value));
            Located::new(
                ExprKind::AssignIndex {
                    object,
                    index,
                    value,
                },
                pos,
            )
        }
        ExprKind::Index { object, index } => {
            let object = Box::new(f.fold_expr(*object));
            let index = Box::new(f.fold_expr(*index));
            Located::new(ExprKind::Index { object, index }, pos)
        }
        ExprKind::Member { object, property } => {
            let object = Box::new(f.fold_expr(*object));
            Located::new(ExprKind::Member { object, property }, pos)
        }
        ExprKind::BinaryOp { op, left, right } => {
            let left = Box::new(f.fold_expr(*left));
            let right = Box::new(f.fold_expr(*right));
            Located::new(ExprKind::BinaryOp { op, left, right }, pos)
        }
        ExprKind::Call { callee, args } => {
            let callee = Box::new(f.fold_expr(*callee));
            let args = args.into_iter().map(|a| f.fold_expr(a)).collect();
            Located::new(ExprKind::Call { callee, args }, pos)
        }
    }
}
