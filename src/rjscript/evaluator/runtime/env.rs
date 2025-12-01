use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use crate::rjscript::{
    ast::{
        block::Block,
        position::Position
    },
    evaluator::{
        errors::EvalError, runtime::value::RJSValue, EvalResult
    }, semantics::types::VarType,
};

pub type EnvRef = Rc<RefCell<Env>>;

#[derive(Clone)]
pub struct UserFunction {
    pub params: Vec<(String, VarType)>,
    pub return_type: VarType,
    pub body: Block,
    pub env: EnvRef,
}

#[derive(Default)]
pub struct Env {
    locals: HashMap<String, (VarType, RJSValue)>,
    functions: Rc<RefCell<HashMap<String, UserFunction>>>,
    parent: Option<EnvRef>,
}

impl Env {
    pub fn new_ref() -> EnvRef {
        let e = Env {
            locals: HashMap::new(),
            functions: Rc::new(RefCell::new(HashMap::new())),
            parent: None,
        };

        let rc = Rc::new(RefCell::new(e));
        rc
    }

    pub fn push_scope(parent: &EnvRef) -> EnvRef {
        let parent_borrow = parent.borrow();
        let child = Env {
            locals: HashMap::new(),
            functions: Rc::clone(&parent_borrow.functions),
            parent: Some(Rc::clone(parent)),
        };
        Rc::new(RefCell::new(child))
    }

    /// Make a new environment that inherits functions/builtins/method tables
    /// from `source` but has no locals (so it doesn't capture surrounding variables).
    pub fn capture_functions_only(source: &EnvRef) -> EnvRef {
        let src = source.borrow();
        let base = Env {
            locals: HashMap::new(), // empty: no closure over surrounding vars
            functions: Rc::new(RefCell::new(src.functions.borrow().clone())),
            parent: None, // don’t chain into original locals
        };
        Rc::new(RefCell::new(base))
    }

    /// Recursively checks for a name in ancestor locals (used for shadowing rules)
    fn has_in_ancestors(&self, name: &str) -> bool {
        if let Some(ref parent_rc) = self.parent {
            let parent = parent_rc.borrow();
            if parent.locals.contains_key(name) {
                true
            } else {
                parent.has_in_ancestors(name)
            }
        } else {
            false
        }
    }

    pub fn define_fn(&mut self, name: &str, func: UserFunction, pos: Position) -> EvalResult<()> {
        let mut tbl = self.functions.borrow_mut();
        if tbl.contains_key(name) {
            return Err(EvalError::VariableAlreadyDeclared(name.to_string(), pos));
        }
        tbl.insert(name.to_string(), func);
        Ok(())
    }

    /// Lookup a user-defined function
    pub fn get_fn(&self, name: &str) -> Option<UserFunction> {
        self.functions.borrow().get(name).cloned()
    }

    pub fn declare_var(
        &mut self,
        name: &str,
        val_type: VarType,
        val: RJSValue,
        pos: Position,
    ) -> EvalResult<()> {
        if self.locals.contains_key(name) {
            return Err(EvalError::VariableAlreadyDeclared(name.to_string(), pos));
        }
        if self.has_in_ancestors(name) {
            return Err(EvalError::VariableAlreadyDeclared(name.to_string(), pos));
        }
        self.locals.insert(name.to_string(), (val_type, val));
        Ok(())
    }

    /// Walk the scope chain, find `name`, and call `f` with (&declared_type, &mut value).
    /// Returns `Some(result)` if the variable exists, else `None`.
    pub fn with_var_slot<F, R>(env: &EnvRef, name: &str, mut f: F) -> Option<R>
    where
        F: FnMut(&VarType, &mut RJSValue) -> R,
    {
        use std::rc::Rc;
        let mut cur = Rc::clone(env);

        loop {
            let has_here = { cur.borrow().locals.contains_key(name) };
            if has_here {
                let mut b = cur.borrow_mut();
                let (decl_ty, val) = b.locals.get_mut(name).unwrap();
                let ty_clone = decl_ty.clone();
                let out = f(&ty_clone, val);
                return Some(out);
            }

            let next = { cur.borrow().parent.as_ref().map(|p| Rc::clone(p)) };
            match next {
                Some(p) => cur = p,
                None => return None,
            }
        }
    }

    pub fn assign_var(&mut self, name: &str, val: RJSValue, pos: Position) -> EvalResult<RJSValue> {
        if let Some((expected_type, _)) = self.get_var(name) {
            if !val.is_type(&expected_type) {
                return Err(EvalError::General(
                    format!(
                        "Type mismatch on assignment to {}: expected {:?}, got {:?}",
                        name, expected_type, val
                    ),
                    pos,
                ));
            }
            self.set_var(name, expected_type.clone(), val.clone(), pos)?;
            Ok(val)
        } else {
            Err(EvalError::UndeclaredVariable(name.to_string(), pos))
        }
    }

    pub fn set_var(
        &mut self,
        name: &str,
        val_type: VarType,
        val: RJSValue,
        pos: Position,
    ) -> EvalResult<()> {
        if self.locals.contains_key(name) {
            self.locals.insert(name.to_string(), (val_type, val));
            Ok(())
        } else if let Some(ref parent_rc) = self.parent {
            parent_rc.borrow_mut().set_var(name, val_type, val, pos)
        } else {
            Err(EvalError::VariableNotFound(name.to_string(), pos))
        }
    }

    pub fn get_var(&self, name: &str) -> Option<(VarType, RJSValue)> {
        if let Some(entry) = self.locals.get(name) {
            Some(entry.clone())
        } else if let Some(ref parent_rc) = self.parent {
            parent_rc.borrow().get_var(name)
        } else {
            None
        }
    }
}
