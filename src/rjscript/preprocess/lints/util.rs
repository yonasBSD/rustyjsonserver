use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::rjscript::{
    ast::{
        block::Block,
        expr::{Expr, ExprKind},
        position::Position,
        stmt::StmtKind,
    },
    semantics::{
        methods::{
            builtin_names_set, receiver_from_vartype, MethodMeta, Receiver, ARRAY_METHODS_META,
            STRING_METHODS_META,
        },
        types::VarType,
    },
};

pub type ScopeRef = Rc<RefCell<Scope>>;

#[derive(Default)]
pub struct Scope {
    vars: HashSet<String>, // let / params
    fns: HashSet<String>,  // function declarations
    parent: Option<ScopeRef>,
}

impl Scope {
    pub fn new_root() -> ScopeRef {
        let mut scope = Scope {
            vars: HashSet::new(),
            fns: HashSet::new(),
            parent: None,
        };
        scope
            .fns
            .extend(builtin_names_set().into_iter().map(|s| s.to_string()));
        Rc::new(RefCell::new(scope))
    }

    /// Create a child scope with `cur` as parent.
    pub fn push_child(cur: &ScopeRef) -> ScopeRef {
        Rc::new(RefCell::new(Scope {
            vars: HashSet::new(),
            fns: HashSet::new(),
            parent: Some(cur.clone()),
        }))
    }

    /// Get the parent of this scope (if any).
    pub fn parent(cur: &ScopeRef) -> Option<ScopeRef> {
        cur.borrow().parent.clone()
    }

    pub fn declare_var(cur: &ScopeRef, name: &str) {
        cur.borrow_mut().vars.insert(name.to_string());
    }

    pub fn declare_fn(cur: &ScopeRef, name: &str) {
        cur.borrow_mut().fns.insert(name.to_string());
    }

    // ---------- “current, then parents (recursive)” checks ----------
    pub fn has_var_in_chain(cur: &ScopeRef, name: &str) -> bool {
        let mut it = Some(cur.clone());
        while let Some(sr) = it {
            let b = sr.borrow();
            if b.vars.contains(name) {
                return true;
            }
            it = b.parent.clone();
        }
        false
    }

    /// Find the scope frame that *declared* the nearest `name` variable.
    /// (Does not consider functions; only variables.)
    pub fn find_decl_scope(cur: &ScopeRef, name: &str) -> Option<ScopeRef> {
        let mut it = Some(cur.clone());
        while let Some(sr) = it {
            let b = sr.borrow();
            if b.vars.contains(name) {
                // found owning frame
                return Some(sr.clone());
            }
            it = b.parent.clone();
        }
        None
    }

    pub fn has_fn_in_chain(cur: &ScopeRef, name: &str) -> bool {
        let mut it = Some(cur.clone());
        while let Some(sr) = it {
            let b = sr.borrow();
            if b.fns.contains(name) {
                return true;
            }
            it = b.parent.clone();
        }
        false
    }
}

#[inline]
pub fn known_method_names_any() -> HashSet<&'static str> {
    ARRAY_METHODS_META
        .iter()
        .map(|(_, m)| m.name)
        .chain(STRING_METHODS_META.iter().map(|(_, m)| m.name))
        .collect()
}

/// Lookup `MethodMeta` for a concrete receiver kind.
#[inline]
pub fn method_meta_for_receiver(receiver: Receiver, name: &str) -> Option<&'static MethodMeta> {
    match receiver {
        Receiver::Array => ARRAY_METHODS_META
            .iter()
            .find(|(_, m)| m.name == name)
            .map(|(_, m)| m),
        Receiver::String => STRING_METHODS_META
            .iter()
            .find(|(_, m)| m.name == name)
            .map(|(_, m)| m),
    }
}

/// Lookup `MethodMeta` using a VarType (Array<T>, String, ...).
#[inline]
pub fn method_meta_for_vartype(ty: &VarType, name: &str) -> Option<&'static MethodMeta> {
    receiver_from_vartype(ty).and_then(|rcv| method_meta_for_receiver(rcv, name))
}

/// Conservative check: if a method name is mutating for *any* known receiver, return true.
#[inline]
pub fn is_mutating_method_any(name: &str) -> bool {
    method_meta_for_receiver(Receiver::Array, name).is_some_and(|m| m.is_mut)
        || method_meta_for_receiver(Receiver::String, name).is_some_and(|m| m.is_mut)
}

/// If `callee` is a bare identifier (free function call), returns its name.
pub fn ident_name_from_callee(callee: &Expr) -> Option<&str> {
    match &callee.kind {
        ExprKind::Ident(name) => Some(name.as_str()),
        _ => None,
    }
}

/// If `callee` is a member access (method call), return (receiver_expr, method_name).
/// E.g., `user.list.push(1)` -> (receiver=`user.list`, method="push")
pub fn receiver_and_method_from_callee<'a>(callee: &'a Expr) -> Option<(&'a Expr, &'a str)> {
    match &callee.kind {
        ExprKind::Member { object, property } => Some((object, property.as_str())),
        _ => None,
    }
}

/// Collect function declarations (`function foo(...) {}`) in a block.
pub fn collect_function_decls(block: &Block) -> HashMap<String, Position> {
    let mut map = HashMap::new();
    for s in &block.stmts {
        if let StmtKind::FunctionDecl { ident, .. } = &s.kind {
            map.insert(ident.clone(), s.pos);
        }
    }
    map
}
