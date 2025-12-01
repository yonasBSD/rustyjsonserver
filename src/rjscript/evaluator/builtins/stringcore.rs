use std::{
    collections::{HashMap},
    sync::{Arc, OnceLock},
};

use crate::rjscript::{
    ast::position::Position, evaluator::{builtins::PureMethodFn, errors::EvalError, runtime::value::RJSValue, EvalResult}, semantics::methods::{StringMethod, STRING_METHODS_META}
};

static STRING_METHODS: OnceLock<Arc<HashMap<String, PureMethodFn>>> = OnceLock::new();

fn string_method_impl(m: StringMethod) -> PureMethodFn {
    match m {
        StringMethod::Length      => string_length,
        StringMethod::Contains => string_contains,
        StringMethod::Split => string_split,
        StringMethod::Substring        => string_substring,
        StringMethod::ToChars       => string_to_chars,
        StringMethod::Replace    => string_replace,
    }
}

pub fn string_methods_table() -> Arc<HashMap<String, PureMethodFn>> {
    STRING_METHODS.get_or_init(|| {
        let mut m = HashMap::new();
        for (enum_key, meta) in STRING_METHODS_META {
            debug_assert!(!meta.is_mut);
            m.insert(meta.name.to_string(), string_method_impl(*enum_key));
        }
        Arc::new(m)
    }).clone()
}

fn string_contains(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    let s = match obj {
        RJSValue::String(s) => s,
        _ => unreachable!(),
    };
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("contains".into(), 1, pos));
    }
    if let RJSValue::String(sub) = &args[0] {
        Ok(RJSValue::Bool(s.contains(sub)))
    } else {
        Err(EvalError::TypeMismatch(
            "contains needs a string".into(),
            pos,
        ))
    }
}

fn string_split(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("split".into(), 1, pos));
    }
    // Ensure the receiver is a string
    let s = match obj {
        RJSValue::String(ref s) => s,
        other => {
            return Err(EvalError::TypeMismatch(
                format!("'split' called on non-string value: {:?}", other),
                pos,
            ));
        }
    };
    // Ensure the delimiter is a string
    let delim = match &args[0] {
        RJSValue::String(d) => d,
        _ => {
            return Err(EvalError::TypeMismatch(
                "Argument to 'split' must be a string".into(),
                pos,
            ));
        }
    };
    // Perform split
    let parts = s
        .split(delim)
        .map(|p| RJSValue::String(p.to_string()))
        .collect::<Vec<_>>();

    Ok(RJSValue::Array(parts))
}

fn string_length(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    let s = match obj {
        RJSValue::String(s) => s,
        _ => unreachable!(),
    };
    if !args.is_empty() {
        return Err(EvalError::WrongNumberOfArguments("length".into(), 0, pos));
    }
    Ok(RJSValue::Number(s.chars().count() as f64))
}

fn string_to_chars(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    if !args.is_empty() {
        return Err(EvalError::WrongNumberOfArguments(
            "to_chars()s".into(),
            0,
            pos,
        ));
    }
    if let RJSValue::String(s) = obj {
        let out = s.chars().map(|c| RJSValue::String(c.to_string())).collect();
        Ok(RJSValue::Array(out))
    } else {
        Err(EvalError::TypeMismatch(
            "to_chars() called on non-string".into(),
            pos,
        ))
    }
}

fn string_replace(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "replace()".into(),
            2,
            pos,
        ));
    }
    if let RJSValue::String(s) = obj {
        match (&args[0], &args[1]) {
            (RJSValue::String(from), RJSValue::String(to)) => {
                Ok(RJSValue::String(s.replace(from, to)))
            }
            _ => Err(EvalError::TypeMismatch(
                "replace() arguments must be strings".into(),
                pos,
            )),
        }
    } else {
        Err(EvalError::TypeMismatch(
            "replace() called on non-string".into(),
            pos,
        ))
    }
}

fn string_substring(obj: &RJSValue, args: &[RJSValue], pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "substring()".into(),
            2,
            pos,
        ));
    }
    if let RJSValue::String(s) = obj {
        if let (RJSValue::Number(st), RJSValue::Number(ed)) = (&args[0], &args[1]) {
            let start = *st as usize;
            let end = *ed as usize;
            if start <= end && end <= s.len() {
                Ok(RJSValue::String(s[start..end].to_string()))
            } else {
                Err(EvalError::General(
                    "substring() indices out of bounds".into(),
                    pos,
                ))
            }
        } else {
            Err(EvalError::TypeMismatch(
                "substring() arguments must be numbers".into(),
                pos,
            ))
        }
    } else {
        Err(EvalError::TypeMismatch(
            "substring() called on non-string".into(),
            pos,
        ))
    }
}