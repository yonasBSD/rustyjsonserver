use std::collections::HashMap;

use crate::rjscript::{
    ast::{
        binop::BinOp,
        block::Block,
        expr::{Expr, ExprKind},
        node::HasPos,
        position::Position,
        stmt::{Stmt, StmtKind},
    }, evaluator::runtime::value::RJSValue, preprocess::lints::{error::LintError, util::{method_meta_for_vartype, receiver_and_method_from_callee}}, semantics::types::VarType
};

pub fn run(block: &Block) -> Vec<LintError> {
    let mut tc = TypeChecker::default();
    tc.check_block(block);
    tc.errors
}

#[derive(Default)]
struct TypeChecker {
    errors: Vec<LintError>,
    scopes: Vec<HashMap<String, VarType>>, // lexical scope stack
}

impl TypeChecker {
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, ty: VarType) {
        if let Some(top) = self.scopes.last_mut() {
            top.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<VarType> {
        for m in self.scopes.iter().rev() {
            if let Some(t) = m.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    /// Assignment compatibility:
    /// - exact type match
    /// - assigning `Undefined` to anything is allowed
    /// - arrays: element types must be assignable recursively
    fn is_assignable(dst: &VarType, src: &VarType) -> bool {
        use VarType::*;
        match (dst, src) {
            // allow default-undefined into anything if you want
            (_, Undefined) => true,

            (Array(d), Array(s)) => match (&**d, &**s) {
                (Any, _) => true,  // vec<any> destination accepts either
                (_, Any) => false, // concrete destination rejects any source
                _ => Self::is_assignable(d, s),
            },
            _ => dst == src,
        }
    }

    fn err(&mut self, pos: Position, msg: String) {
        self.errors.push(LintError::new(pos.into(), msg));
    }

    pub fn check_block(&mut self, b: &Block) {
        self.push_scope();
        for s in &b.stmts {
            self.check_stmt(s);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let { name, ty, init } => {
                if let Some(e) = init {
                    if let Some(rhs_ty) = self.infer_expr(e) {
                        // Special-case: allow `let xs: vec<T> = []` for any T.
                        let is_empty_array_literal = matches!(
                            (&ty, &rhs_ty, &e.kind),
                            (
                                VarType::Array(_),
                                VarType::Array(inner_src),
                                ExprKind::Array(items)
                            ) if **inner_src == VarType::Any && items.is_empty()
                        );

                        if !is_empty_array_literal && !Self::is_assignable(ty, &rhs_ty) {
                            self.err(
                                s.pos(),
                                format!(
                                    "Type mismatch: cannot assign {} to variable '{}' of type {}",
                                    rhs_ty, name, ty
                                ),
                            );
                        }
                    }
                }
                // Declare after checking so `let x: T = x;` still sees outer x, if any.
                self.declare(name, ty.clone());
            }

            StmtKind::ExprStmt(e) => {
                // catches assignment expressions like `a = ...` or `arr[i] = ...`
                self.infer_expr(e);
            }

            StmtKind::Return(e) => {
                self.infer_expr(e);
            }
            StmtKind::ReturnStatus { status, value } => {
                self.infer_expr(status);
                self.infer_expr(value);
            }

            StmtKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                self.infer_expr(condition);
                self.check_block(then_block);
                if let Some(b) = else_block {
                    self.check_block(b);
                }
            }

            StmtKind::For {
                init,
                condition,
                increment,
                body,
            } => {
                self.push_scope(); // for-loop scope
                if let Some(s0) = init {
                    self.check_stmt(s0);
                }
                self.infer_expr(condition);
                if let Some(inc) = increment {
                    self.infer_expr(inc);
                }
                self.check_block(body);
                self.pop_scope();
            }

            StmtKind::FunctionDecl { params, body, .. } => {
                self.push_scope();
                for (pname, pty) in params {
                    self.declare(pname, pty.clone());
                }
                self.check_block(body);
                self.pop_scope();
            }

            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::Switch {
                condition,
                cases,
                default,
            } => {
                self.infer_expr(condition);
                for (e, b) in cases {
                    self.infer_expr(e);
                    self.check_block(b);
                }
                if let Some(b) = default {
                    self.check_block(b);
                }
            }
        }
    }

    /// Returns Some(type) when inferrable, None when unknown.
    fn infer_expr(&mut self, e: &Expr) -> Option<VarType> {
        match &e.kind {
            ExprKind::Literal(lit) => Some(RJSValue::from_literal(lit.clone()).to_type()),
            ExprKind::TypeLiteral(_) => None, // not a runtime value
            ExprKind::Template(_) => Some(VarType::String),

            ExprKind::Ident(name) => self.lookup(name),

            ExprKind::ObjectLiteral { .. } => Some(VarType::Object),

            ExprKind::Array(items) => {
                use VarType::*;
                let mut elem: Option<VarType> = None;
                for it in items {
                    if let Some(t) = self.infer_expr(it) {
                        match &mut elem {
                            None => elem = Some(t),
                            Some(prev) if *prev == t => {}
                            Some(_) => {
                                elem = Some(Any);
                                break;
                            } // <- mixed => any
                        }
                    } else {
                        elem = Some(Any);
                        break;
                    } // unknown => any
                }
                Some(Array(Box::new(elem.unwrap_or(Any))))
            }

            ExprKind::RequestField(_) => None, // dynamic / unknown statically

            // Assignments:
            //  - var = value       : check against declared var type
            //  - obj.prop = value  : unknown statically (we still walk RHS)
            //  - arr[idx] = value  : if arr is Array(T) and idx is Number, enforce value : T
            ExprKind::AssignVar { name, value } => {
                let rhs = self.infer_expr(value.as_ref());
                if let Some(rhs_ty) = &rhs {
                    if let Some(var_ty) = self.lookup(name) {
                        if !Self::is_assignable(&var_ty, rhs_ty) {
                            self.err(
                                e.pos(),
                                format!(
                                    "Type mismatch in assignment to '{}': cannot assign {} to {}",
                                    name, rhs_ty, var_ty
                                ),
                            );
                        }
                    }
                }
                rhs
            }
            ExprKind::AssignMember { value, .. } => self.infer_expr(value.as_ref()),
            ExprKind::AssignIndex {
                object,
                index,
                value,
            } => {
                let arr_ty = self.infer_expr(object.as_ref());
                let idx_ty = self.infer_expr(index.as_ref());
                let rhs_ty = self.infer_expr(value.as_ref());

                // Match on &Option<VarType> so nothing is moved
                if let (Some(VarType::Array(inner)), Some(VarType::Number), Some(vt)) =
                    (arr_ty.as_ref(), idx_ty.as_ref(), rhs_ty.as_ref())
                {
                    if !Self::is_assignable(inner.as_ref(), vt) {
                        self.err(
                            e.pos(),
                            format!(
                                "Type mismatch: cannot assign element of type {} into array of {}",
                                vt,
                                inner.as_ref()
                            ),
                        );
                    }
                }

                rhs_ty
            }

            // Reads
            ExprKind::Member { .. } => None,
            ExprKind::Index { object, index } => {
                let obj_ty = self.infer_expr(object.as_ref());
                let idx_ty = self.infer_expr(index.as_ref());
                match (obj_ty, idx_ty) {
                    (Some(VarType::Array(inner)), Some(VarType::Number)) => Some(*inner), // arr[num] : inner
                    (Some(VarType::Object), Some(VarType::String)) => None, // unknown field type
                    _ => None,
                }
            }

            // Calls: we don't track user function return types yet,
            // but we can infer Number for known ".length()" etc. via method meta.
            ExprKind::Call { callee, args } => {
                // visit args regardless
                for a in args {
                    self.infer_expr(a);
                }

                // If it's a method call, try lightweight inference via meta (e.g., length -> Number)
                if let Some((recv_expr, method)) = receiver_and_method_from_callee(callee) {
                    if let Some(recv_ty) = self.infer_expr(recv_expr) {
                        if let Some(meta) = method_meta_for_vartype(&recv_ty, method) {
                            if meta.returns_number {
                                return Some(VarType::Number);
                            }
                        }
                    }
                }

                None
            }

            // Binary operators
            ExprKind::BinaryOp { op, left, right } => {
                let lt = self.infer_expr(left.as_ref());
                let rt = self.infer_expr(right.as_ref());
                self.infer_binop(*op, lt, rt, e)
            }
        }
    }

    fn infer_binop(
        &mut self,
        op: BinOp,
        lt: Option<VarType>,
        rt: Option<VarType>,
        at: &Expr,
    ) -> Option<VarType> {
        use BinOp::*;
        use VarType::*;
        match op {
            Add => match (lt, rt) {
                (Some(Number), Some(Number)) => Some(Number),
                (Some(String), Some(String)) => Some(String),
                (Some(a), Some(b)) => {
                    self.err(
                        at.pos(),
                        format!("Invalid '+' operand types {} and {}", a, b),
                    );
                    None
                }
                _ => None,
            },
            Sub | Mul | Div | Rem => match (lt, rt) {
                (Some(Number), Some(Number)) => Some(Number),
                (Some(a), Some(b)) => {
                    self.err(
                        at.pos(),
                        format!("Numeric operator requires numbers, got {} and {}", a, b),
                    );
                    None
                }
                _ => None,
            },
            Eq | Ne => Some(Bool),
            Lt | Le | Gt | Ge => match (lt, rt) {
                (Some(Number), Some(Number)) => Some(Bool),
                (Some(a), Some(b)) => {
                    self.err(
                        at.pos(),
                        format!("Order comparison requires numbers, got {} and {}", a, b),
                    );
                    None
                }
                _ => None,
            },
            And | Or => {
                // Runtime truthiness accepts bool/number.
                let bad = |t: &VarType| !matches!(t, Bool | Number);
                if let (Some(l), Some(r)) = (lt.clone(), rt.clone()) {
                    if bad(&l) || bad(&r) {
                        self.err(
                            at.pos(),
                            format!("Logical operator requires bool/number, got {} and {}", l, r),
                        );
                    }
                }
                Some(Bool)
            }
        }
    }
}
