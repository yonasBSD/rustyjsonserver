use std::collections::HashMap;

use crate::rjscript::{
    ast::{
        binop::BinOp,
        block::Block,
        expr::{Expr, ExprKind, TemplatePart},
        node::HasPos,
        position::Position,
        request::RequestFieldType,
        stmt::{Stmt, StmtKind},
    },
    preprocess::lints::{
        error::LintError, must_return::block_returns, util::{
            ident_name_from_callee,
            known_method_names_any,
            method_meta_for_receiver,
            receiver_and_method_from_callee,
        }
    },
    semantics::{methods::Receiver, types::VarType},
};


pub fn run(block: &Block) -> Vec<LintError> {
    let mut l = ReqTypeGuard::default();
    l.check_block(block, &mut Facts::default(), &mut Scope::default());
    l.errors
}

/// If `e` is a call like `<object>.length()`, return the `<object>` expr.
fn as_length_call_on(e: &Expr) -> Option<&Expr> {
    if let ExprKind::Call { callee, args } = &e.kind {
        if args.is_empty() {
            if let Some((object, method)) = receiver_and_method_from_callee(callee) {
                if method == "length" && known_method_names_any().contains("length") {
                    return Some(object);
                }
            }
        }
    }
    None
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct ExprKey(String);

fn fingerprint_expr(e: &Expr) -> ExprKey {
    fn go(e: &Expr, s: &mut String) {
        use ExprKind::*;
        match &e.kind {
            Literal(v) => s.push_str(&format!("Lit({:?})", v)),
            Ident(n) => s.push_str(&format!("Var({})", n)),
            TypeLiteral(t) => s.push_str(&format!("Type({:?})", t)),
            Template(parts) => {
                s.push_str("Tpl(");
                for p in parts {
                    match p {
                        TemplatePart::Text(t) => s.push_str(&format!("T{:?};", t)),
                        TemplatePart::Expr(e2) => {
                            s.push('E');
                            go(e2, s);
                            s.push(';');
                        }
                    }
                }
                s.push(')');
            }
            ObjectLiteral { fields } => {
                s.push_str("Obj{");
                for (k, v) in fields {
                    s.push_str(&format!("{}:", k));
                    go(v, s);
                    s.push(';');
                }
                s.push('}');
            }
            Array(items) => {
                s.push('[');
                for it in items {
                    go(it, s);
                    s.push(';');
                }
                s.push(']');
            }
            RequestField(RequestFieldType::BodyField) => {
                s.push_str("ReqBody()");
            }
            RequestField(RequestFieldType::ParamField) => {
                s.push_str("ReqParam()")
            }
            RequestField(RequestFieldType::QueryField) => {
                s.push_str("ReqQuery()")
            }
            RequestField(RequestFieldType::HeadersField) => {
                s.push_str("ReqHeader()")
            }
            Member { object, property } => {
                s.push_str("Mem(");
                go(object, s);
                s.push('.');
                s.push_str(property);
                s.push(')');
            }
            Index { object, index } => {
                s.push_str("Idx(");
                go(object, s);
                s.push('[');
                go(index, s);
                s.push_str("])");
            }
            BinaryOp { op, left, right } => {
                s.push_str(&format!("Bin({:?},", op));
                go(left, s);
                s.push(',');
                go(right, s);
                s.push(')');
            }
            Call { callee, args } => {
                s.push_str("Call(");
                go(callee, s);
                s.push('(');
                for a in args {
                    go(a, s);
                    s.push(';');
                }
                s.push_str("))");
            }
            AssignVar { name, value } => {
                s.push_str(&format!("AssignVar({},", name));
                go(value, s);
                s.push(')');
            }
            AssignMember {
                object,
                property,
                value,
            } => {
                s.push_str("AssignMem(");
                go(object, s);
                s.push('.');
                s.push_str(property);
                s.push(',');
                go(value, s);
                s.push(')');
            }
            AssignIndex {
                object,
                index,
                value,
            } => {
                s.push_str("AssignIdx(");
                go(object, s);
                s.push('[');
                go(index, s);
                s.push_str("],");
                go(value, s);
                s.push(')');
            }
        }
    }
    let mut s = String::new();
    go(e, &mut s);
    ExprKey(s)
}

#[derive(Default, Clone)]
struct Facts {
    // Known types for specific expressions along the current path (by ExprKey).
    map: HashMap<ExprKey, VarType>,
}
impl Facts {
    fn get(&self, k: &ExprKey) -> Option<&VarType> {
        self.map.get(k)
    }
    fn set(&mut self, k: ExprKey, t: VarType) {
        self.map.insert(k, t);
    }
    fn has_type(&self, k: &ExprKey, want: &VarType) -> bool {
        self.get(k).map(|t| t == want).unwrap_or(false)
    }
}

#[derive(Default, Clone)]
struct Scope {
    stack: Vec<HashMap<String, VarType>>,
}
impl Scope {
    fn push(&mut self) {
        self.stack.push(HashMap::new());
    }
    fn pop(&mut self) {
        self.stack.pop();
    }
    fn declare(&mut self, name: &str, ty: VarType) {
        if let Some(top) = self.stack.last_mut() {
            top.insert(name.to_string(), ty);
        }
    }
    fn lookup(&self, name: &str) -> Option<VarType> {
        for m in self.stack.iter().rev() {
            if let Some(t) = m.get(name) {
                return Some(t.clone());
            }
        }
        None
    }
}

#[derive(Default)]
struct ReqTypeGuard {
    errors: Vec<LintError>,
}
impl ReqTypeGuard {
    fn err(&mut self, pos: Position, msg: String) {
        self.errors.push(LintError::new(pos, msg));
    }

    fn check_block(&mut self, b: &Block, facts: &mut Facts, scope: &mut Scope) {
        scope.push();
        let mut after_facts = facts.clone();

        for s in &b.stmts {
            self.check_stmt(s, &mut after_facts, scope);
        }

        *facts = after_facts;
        scope.pop();
    }

    fn check_stmt(&mut self, s: &Stmt, facts: &mut Facts, scope: &mut Scope) {
        match &s.kind {
            // let x: T = <expr>;
            StmtKind::Let { name, ty, init } => {
                if let Some(rhs) = init {
                    self.enforce_guard_if_req(rhs, ty, s.pos(), facts);
                    // numeric usage inside initializer
                    enforce_numeric_usage_on_expr(self, rhs, facts);
                }
                scope.declare(name, ty.clone());
            }

            // Expression stmt
            StmtKind::ExprStmt(e) => {
                enforce_numeric_usage_on_expr(self, e, facts);
                self.check_expr_for_assignments(e, facts, scope);
            }

            StmtKind::Return(e) => {
                enforce_numeric_usage_on_expr(self, e, facts);
                self.check_expr_for_assignments(e, facts, scope);
            }
            StmtKind::ReturnStatus { status, value } => {
                enforce_numeric_usage_on_expr(self, status, facts);
                enforce_numeric_usage_on_expr(self, value, facts);
                self.check_expr_for_assignments(status, facts, scope);
                self.check_expr_for_assignments(value, facts, scope);
            }

            // if (…) { … } [ else { … } ]
            StmtKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                // Enforce numeric usage with &&-guard awareness in the condition
                enforce_numeric_usage_in_condition(self, condition, facts);

                let guard = extract_type_guard(condition);

                // Branch facts
                let mut then_facts = facts.clone();
                let mut else_facts = facts.clone();

                if let Some((key, ty, GuardKind::Eq)) = &guard {
                    then_facts.set(key.clone(), ty.clone());
                } else if let Some((key, ty, GuardKind::Ne)) = &guard {
                    // `toType(expr) != T` → inside else, expr is T
                    else_facts.set(key.clone(), ty.clone());
                }

                // Check branches
                self.check_block(then_block, &mut then_facts, scope);
                if let Some(else_b) = else_block {
                    self.check_block(else_b, &mut else_facts, scope);
                }

                // Facts after the if:
                if let Some((key, ty, kind)) = guard {
                    match kind {
                        GuardKind::Eq => {
                            if else_block.as_ref().map(block_returns).unwrap_or(false) {
                                facts.set(key, ty);
                            }
                        }
                        GuardKind::Ne => {
                            if block_returns(then_block) {
                                facts.set(key, ty);
                            }
                        }
                    }
                }
            }

            StmtKind::For {
                init,
                condition,
                increment,
                body,
            } => {
                scope.push();
                if let Some(s0) = init.as_deref() {
                    self.check_stmt(s0, facts, scope);
                }
                enforce_numeric_usage_in_condition(self, condition, facts);
                self.check_expr_for_assignments(condition, facts, scope);
                if let Some(inc) = increment {
                    self.check_expr_for_assignments(inc, facts, scope);
                }
                let mut inner = facts.clone();
                self.check_block(body, &mut inner, scope);
                scope.pop();
            }

            // switch: facts don't flow between cases; just check inside.
            StmtKind::Switch {
                condition,
                cases,
                default,
            } => {
                enforce_numeric_usage_on_expr(self, condition, facts);
                self.check_expr_for_assignments(condition, facts, scope);
                for (e, b) in cases {
                    enforce_numeric_usage_on_expr(self, e, facts);
                    self.check_expr_for_assignments(e, facts, scope);
                    let mut inner = facts.clone();
                    self.check_block(b, &mut inner, scope);
                }
                if let Some(b) = default {
                    let mut inner = facts.clone();
                    self.check_block(b, &mut inner, scope);
                }
            }

            StmtKind::FunctionDecl { params, body, .. } => {
                scope.push();
                for (pname, pty) in params {
                    scope.declare(pname, pty.clone());
                }
                let mut inner = Facts::default(); // do not inherit outer facts
                self.check_block(body, &mut inner, scope);
                scope.pop();
            }

            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    /// Walk an expression tree and check any assignment subexpressions it contains.
    fn check_expr_for_assignments(&mut self, e: &Expr, facts: &mut Facts, scope: &mut Scope) {
        enforce_method_usage_on_expr(self, e, facts);

        match &e.kind {
            ExprKind::AssignVar { name, value } => {
                if let Some(lhs_ty) = scope.lookup(name) {
                    self.enforce_guard_if_req(value, &lhs_ty, e.pos(), facts);
                }
                enforce_numeric_usage_on_expr(self, value, facts);
                // Recurse into RHS in case of nested assignments
                self.check_expr_for_assignments(value, facts, scope);
            }
            // Recurse through other constructs
            ExprKind::BinaryOp { left, right, .. } => {
                self.check_expr_for_assignments(left, facts, scope);
                self.check_expr_for_assignments(right, facts, scope);
            }
            ExprKind::Index { object, index } => {
                self.check_expr_for_assignments(object, facts, scope);
                self.check_expr_for_assignments(index, facts, scope);
            }
            ExprKind::Member { object, .. } => {
                self.check_expr_for_assignments(object, facts, scope);
            }
            ExprKind::Call { callee, args } => {
                self.check_expr_for_assignments(callee, facts, scope);
                for a in args {
                    self.check_expr_for_assignments(a, facts, scope);
                }
            }
            ExprKind::Array(items) => {
                for it in items {
                    self.check_expr_for_assignments(it, facts, scope);
                }
            }
            ExprKind::ObjectLiteral { fields } => {
                for (_, ex) in fields {
                    self.check_expr_for_assignments(ex, facts, scope);
                }
            }
            ExprKind::Template(parts) => {
                for p in parts {
                    if let TemplatePart::Expr(ex) = p {
                        self.check_expr_for_assignments(ex, facts, scope);
                    }
                }
            }
            _ => {}
        }
    }

    fn enforce_guard_if_req(&mut self, val: &Expr, want: &VarType, at: Position, facts: &Facts) {
        if !val.is_request_derived() {
            return;
        }

        let key = fingerprint_expr(val);
        let guarded_same = facts.has_type(&key, want);
        let guarded_via_method = is_guarded_req_method_result(val, want, facts);

        if !guarded_same && !guarded_via_method {
            self.err(
                at,
                format!(
                    "Assigning request-derived value to type {:?} requires a prior type check for the same expression \
                     (e.g., `if (toType(<expr>) == {:?}) {{ ... }}` or the negated check with early return)",
                    want, want
                ),
            );
        }
    }

    /// Helper used by numeric-usage enforcement.
    fn require_guard_type_if_request(
        &mut self,
        val: &Expr,
        want: &VarType,
        at: Position,
        facts: &Facts,
    ) {
        if val.is_request_derived() {
            let key = fingerprint_expr(val);
            if !facts.has_type(&key, want) {
                self.err(
                    at,
                    format!(
                        "Using request-derived value in a numeric operation requires a prior type check to {:?} \
                         (e.g., `if (toType(<expr>) == {:?}) {{ ... }}` or the negated early-return form)",
                        want, want
                    ),
                );
            }
        }
    }
}

/// Require that a request-derived receiver is guarded to an allowed type for that method.
fn require_guard_for_method_on_request(
    l: &mut ReqTypeGuard,
    object: &Expr,
    method: &str,
    at: Position,
    facts: &Facts,
) {
    if !object.is_request_derived() {
        return;
    }

    // Check support per receiver kind using util meta
    let on_str = method_meta_for_receiver(Receiver::String, method).is_some();
    let on_arr = method_meta_for_receiver(Receiver::Array, method).is_some();

    if !on_str && !on_arr {
        // Unknown method; other lints (unknown_calls) handle it.
        return;
    }

    let key = fingerprint_expr(object);
    let mut allowed_types: Vec<VarType> = Vec::new();
    if on_str {
        allowed_types.push(VarType::String);
    }
    if on_arr {
        // receiver-guard to "array of any"
        allowed_types.push(VarType::Array(Box::new(VarType::Any)));
    }

    // We accept any matching guard on the same receiver expression
    if !allowed_types.iter().any(|t| facts.has_type(&key, t)) {
        let want_desc = if on_str && on_arr {
            "str or vec"
        } else if on_str {
            "str"
        } else {
            "vec"
        };

        l.err(
            at,
            format!(
                "Calling method '{}' on a request-derived value requires a prior type check \
                 guarding the receiver to {} (e.g., `if (toType(<expr>) == {}) {{ ... }}`)",
                method,
                want_desc,
                if on_str && !on_arr { "str" } else { "vec" }
            ),
        );
    }
}

/// Enforce numeric requirement for `<`, `<=`, `>`, `>=`, `-`, `*`, `/`, `%`.
fn enforce_numeric_usage_on_expr(l: &mut ReqTypeGuard, e: &Expr, facts: &Facts) {
    use ExprKind::*;
    match &e.kind {
        BinaryOp { op, left, right } => {
            match op {
                BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Rem => {
                    // LEFT operand
                    if let Some(obj) = as_length_call_on(left) {
                        // `.length()` is numeric if the receiver is an array or string under guard
                        require_array_or_string_receiver_for_length(l, obj, left.pos(), facts);
                    } else {
                        l.require_guard_type_if_request(left, &VarType::Number, left.pos(), facts);
                    }

                    // RIGHT operand
                    if let Some(obj) = as_length_call_on(right) {
                        require_array_or_string_receiver_for_length(l, obj, right.pos(), facts);
                    } else {
                        l.require_guard_type_if_request(
                            right,
                            &VarType::Number,
                            right.pos(),
                            facts,
                        );
                    }
                }
                _ => {
                    enforce_numeric_usage_on_expr(l, left, facts);
                    enforce_numeric_usage_on_expr(l, right, facts);
                }
            }
        }
        Member { object, .. } => enforce_numeric_usage_on_expr(l, object, facts),
        Index { object, index } => {
            enforce_numeric_usage_on_expr(l, object, facts);
            enforce_numeric_usage_on_expr(l, index, facts);
        }
        Call { callee, args } => {
            enforce_numeric_usage_on_expr(l, callee, facts);
            for a in args {
                enforce_numeric_usage_on_expr(l, a, facts);
            }
        }
        Array(items) => {
            for it in items {
                enforce_numeric_usage_on_expr(l, it, facts);
            }
        }
        ObjectLiteral { fields } => {
            for (_, ex) in fields {
                enforce_numeric_usage_on_expr(l, ex, facts);
            }
        }
        Template(parts) => {
            for p in parts {
                if let TemplatePart::Expr(ex) = p {
                    enforce_numeric_usage_on_expr(l, ex, facts);
                }
            }
        }
        AssignVar { value, .. } => enforce_numeric_usage_on_expr(l, value, facts),
        AssignMember { value, .. } | AssignIndex { value, .. } => {
            enforce_numeric_usage_on_expr(l, value, facts)
        }
        _ => {}
    }
}

/// Like `enforce_numeric_usage_on_expr`, but aware of `&&` guards in `if` conditions`:
/// if ( toType(expr)==num && ( expr < 5 ) )
fn enforce_numeric_usage_in_condition(l: &mut ReqTypeGuard, cond: &Expr, facts: &Facts) {
    use ExprKind::*;
    match &cond.kind {
        BinaryOp {
            op: BinOp::And,
            left,
            right,
        } => {
            // Evaluate left normally
            enforce_numeric_usage_on_expr(l, left, facts);
            // If left establishes a guard, use it when checking the right
            let mut facts_with_guard = facts.clone();
            if let Some((key, ty, GuardKind::Eq)) = extract_type_guard(left) {
                facts_with_guard.set(key, ty);
            }
            enforce_numeric_usage_in_condition(l, right, &facts_with_guard);
        }
        // For OR or anything else, just check normally
        _ => enforce_numeric_usage_on_expr(l, cond, facts),
    }
}

/// Require `<object>.length()` receiver is guarded as vec<...> or string when request-derived.
fn require_array_or_string_receiver_for_length(
    l: &mut ReqTypeGuard,
    object: &Expr,
    at: Position,
    facts: &Facts,
) {
    if !object.is_request_derived() {
        return;
    }

    // "length" is defined on String and Array in our meta
    let on_str = method_meta_for_receiver(Receiver::String, "length").is_some();
    let on_arr = method_meta_for_receiver(Receiver::Array, "length").is_some();

    if !on_str && !on_arr {
        return;
    }

    let key = fingerprint_expr(object);
    let guarded = (on_str && facts.has_type(&key, &VarType::String))
        || (on_arr && facts.has_type(&key, &VarType::Array(Box::new(VarType::Any))));

    if !guarded {
        l.err(
            at,
            "Using `.length()` on a request-derived value requires a prior type check \
             guarding the receiver to str or vec."
                .to_string(),
        );
    }
}

/// Identify a `toType(expr)` equality/inequality check and return the guarded expression's key.
///   - toType(expr) == TypeLiteral(T)   (or flipped)
///   - toType(expr) != TypeLiteral(T)   (or flipped)
/// Also: if the condition is an `AND` chain, any conjunct guard suffices.
#[derive(Copy, Clone)]
enum GuardKind {
    Eq,
    Ne,
}

fn as_to_type_call(e: &Expr) -> Option<&Expr> {
    if let ExprKind::Call { callee, args } = &e.kind {
        if args.len() == 1 {
            if let Some(name) = ident_name_from_callee(callee) {
                if name == "toType" {
                    return Some(&args[0]);
                }
            }
        }
    }
    None
}

fn as_type_literal(e: &Expr) -> Option<VarType> {
    if let ExprKind::TypeLiteral(t) = &e.kind {
        Some(t.clone())
    } else {
        None
    }
}

fn extract_type_guard(cond: &Expr) -> Option<(ExprKey, VarType, GuardKind)> {
    use ExprKind::*;
    if let BinaryOp { op, left, right } = &cond.kind {
        if matches!(op, BinOp::Eq | BinOp::Ne) {
            // toType(x) <op> TypeLiteral(T)
            if let (Some(arg), Some(ty)) = (as_to_type_call(left), as_type_literal(right)) {
                if arg.is_request_derived() {
                    let key = fingerprint_expr(arg);
                    return Some((
                        key,
                        ty,
                        if *op == BinOp::Eq {
                            GuardKind::Eq
                        } else {
                            GuardKind::Ne
                        },
                    ));
                }
            }
            // TypeLiteral(T) <op> toType(x)
            if let (Some(ty), Some(arg)) = (as_type_literal(left), as_to_type_call(right)) {
                if arg.is_request_derived() {
                    let key = fingerprint_expr(arg);
                    return Some((
                        key,
                        ty,
                        if *op == BinOp::Eq {
                            GuardKind::Eq
                        } else {
                            GuardKind::Ne
                        },
                    ));
                }
            }
        }
        // Conjunction: recurse
        if let BinOp::And = op {
            if let Some(hit) = extract_type_guard(left) {
                return Some(hit);
            }
            if let Some(hit) = extract_type_guard(right) {
                return Some(hit);
            }
        }
    }
    None
}

/// True if `expr` is a method call on a request-derived receiver that is already
/// guarded to a type compatible with `expected_ty` (e.g., str or vec).
fn is_guarded_req_method_result(expr: &Expr, expected_ty: &VarType, facts: &Facts) -> bool {
    let ExprKind::Call { callee, .. } = &expr.kind else {
        return false;
    };

    let Some((recv, method)) = receiver_and_method_from_callee(callee) else {
        return false;
    };
    if !recv.is_request_derived() {
        return false;
    }

    match expected_ty {
        VarType::String => {
            // Receiver must be guarded as String AND the method must exist on String.
            let recv_fp = fingerprint_expr(recv);
            facts.has_type(&recv_fp, &VarType::String)
                && method_meta_for_receiver(Receiver::String, method).is_some()
        }
        VarType::Array(_) => {
            // Receiver must be guarded as Array AND the method must exist on Array.
            let recv_fp = fingerprint_expr(recv);
            facts.has_type(&recv_fp, &VarType::Array(Box::new(VarType::Any)))
                && method_meta_for_receiver(Receiver::Array, method).is_some()
        }
        _ => false, // for other target types keep the old strict behavior
    }
}

/// Enforce method-usage guards (ANY method, e.g. `.foo()`, `.bar()`) anywhere in an expression tree.
fn enforce_method_usage_on_expr(l: &mut ReqTypeGuard, e: &Expr, facts: &Facts) {
    use ExprKind::*;
    match &e.kind {
        Call { callee, args } => {
            // If this is a method call (obj.method(...)), enforce receiver guard:
            if let Some((object, method)) = receiver_and_method_from_callee(callee) {
                require_guard_for_method_on_request(l, object, method, e.pos(), facts);
                // Still walk into the callee in case there are nested/member calls
                enforce_method_usage_on_expr(l, callee, facts);
            }
            for a in args {
                enforce_method_usage_on_expr(l, a, facts);
            }
        }

        // Recurse through structure
        BinaryOp { left, right, .. } => {
            enforce_method_usage_on_expr(l, left, facts);
            enforce_method_usage_on_expr(l, right, facts);
        }
        Member { object, .. } => enforce_method_usage_on_expr(l, object, facts),
        Index { object, index } => {
            enforce_method_usage_on_expr(l, object, facts);
            enforce_method_usage_on_expr(l, index, facts);
        }
        AssignVar { value, .. } => enforce_method_usage_on_expr(l, value, facts),
        AssignMember { object, value, .. } => {
            enforce_method_usage_on_expr(l, object, facts);
            enforce_method_usage_on_expr(l, value, facts);
        }
        AssignIndex { object, index, value } => {
            enforce_method_usage_on_expr(l, object, facts);
            enforce_method_usage_on_expr(l, index, facts);
            enforce_method_usage_on_expr(l, value, facts);
        }
        Array(items) => {
            for it in items {
                enforce_method_usage_on_expr(l, it, facts);
            }
        }
        ObjectLiteral { fields } => {
            for (_, ex) in fields {
                enforce_method_usage_on_expr(l, ex, facts);
            }
        }
        Template(parts) => {
            for p in parts {
                if let TemplatePart::Expr(ex) = p {
                    enforce_method_usage_on_expr(l, ex, facts);
                }
            }
        }
        // Literals, Vars, TypeLiteral, RequestField → nothing special
        _ => {}
    }
}
