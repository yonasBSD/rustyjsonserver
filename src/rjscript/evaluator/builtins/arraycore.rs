use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use crate::rjscript::{
    ast::position::Position,
    evaluator::{
        builtins::{MutMethodFn, PureMethodFn}, errors::EvalError, runtime::value::RJSValue, EvalResult
    },
    semantics::methods::{ArrayMethod, ARRAY_METHODS_META}
};

static ARRAY_METHODS: OnceLock<Arc<HashMap<String, PureMethodFn>>> = OnceLock::new();
static MUT_ARRAY_METHODS: OnceLock<Arc<HashMap<String, MutMethodFn>>> = OnceLock::new();

fn array_method_pure_impl(m: ArrayMethod) -> PureMethodFn {
    match m {
        ArrayMethod::Length => array_length,
        ArrayMethod::Push | ArrayMethod::Remove | ArrayMethod::RemoveAt => {
            unreachable!("mut array method asked as pure")
        }
    }
}

fn array_method_mut_impl(m: ArrayMethod) -> MutMethodFn {
    match m {
        ArrayMethod::Push => array_push,
        ArrayMethod::Remove => array_remove,
        ArrayMethod::RemoveAt => array_remove_at,
        ArrayMethod::Length => unreachable!("pure array method asked as mut"),
    }
}

pub fn array_methods_table() -> Arc<HashMap<String, PureMethodFn>> {
    ARRAY_METHODS
        .get_or_init(|| {
            let mut m = HashMap::new();
            for (enum_key, meta) in ARRAY_METHODS_META {
                if !meta.is_mut {
                    m.insert(meta.name.to_string(), array_method_pure_impl(*enum_key));
                }
            }
            Arc::new(m)
        })
        .clone()
}

pub fn array_mut_methods_table() -> Arc<HashMap<String, MutMethodFn>> {
    MUT_ARRAY_METHODS
        .get_or_init(|| {
            let mut m = HashMap::new();
            for (enum_key, meta) in ARRAY_METHODS_META {
                if meta.is_mut {
                    m.insert(meta.name.to_string(), array_method_mut_impl(*enum_key));
                }
            }
            Arc::new(m)
        })
        .clone()
}

fn array_length(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    let arr = match obj {
        RJSValue::Array(s) => s,
        _ => unreachable!(),
    };
    if !args.is_empty() {
        return Err(EvalError::WrongNumberOfArguments("length".into(), 0, pos));
    }
    Ok(RJSValue::Number(arr.len() as f64))
}

fn array_push(target: &mut RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("push".into(), 1, pos));
    }
    if let RJSValue::Array(ref mut arr) = target {
        arr.push(args[0].clone());
        Ok(RJSValue::Number(arr.len() as f64))
    } else {
        Err(EvalError::General("push() called on non-array".into(), pos))
    }
}

fn array_remove(target: &mut RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("remove".into(), 1, pos));
    }
    if let RJSValue::Array(ref mut arr) = target {
        if let Some(ix) = arr.iter().position(|v| v == &args[0]) {
            arr.remove(ix);
            Ok(RJSValue::Bool(true))
        } else {
            Ok(RJSValue::Bool(false))
        }
    } else {
        Err(EvalError::General(
            "remove() called on non-array".into(),
            pos,
        ))
    }
}

fn array_remove_at(
    target: &mut RJSValue,
    args: &[RJSValue],
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("removeAt".into(), 1, pos));
    }
    let idx = match args[0] {
        RJSValue::Number(n) if n >= 0.0 => n as usize,
        RJSValue::Number(_) => {
            return Err(EvalError::General(
                "removeAt() index cannot be negative".into(),
                pos,
            ));
        }
        ref other => {
            return Err(EvalError::TypeMismatch(
                format!("removeAt() index must be number, got {:?}", other),
                pos,
            ));
        }
    };
    if let RJSValue::Array(ref mut arr) = target {
        if idx < arr.len() {
            Ok(arr.remove(idx))
        } else {
            Err(EvalError::General(
                format!("removeAt() index {} out of bounds (len={})", idx, arr.len()),
                pos,
            ))
        }
    } else {
        Err(EvalError::General(
            "removeAt() called on non-array".into(),
            pos,
        ))
    }
}
