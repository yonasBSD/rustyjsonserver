use std::collections::HashMap;

use crate::{rjscript::{
    ast::{literal::Literal, position::Position},
    evaluator::{EvalResult, errors::EvalError},
    semantics::types::VarType,
}, rjsdb::DbValue};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq)]
pub enum RJSValue {
    Number(f64),
    String(String),
    Bool(bool),
    Array(Vec<RJSValue>),
    Object(HashMap<String, RJSValue>),
    Type(VarType),
    Undefined,
}

impl RJSValue {
    /// Check whether this value conforms to the requested VarType.
    pub fn is_type(&self, var_type: &VarType) -> bool {
        match (self, var_type) {
            (RJSValue::Bool(_), VarType::Bool) => true,
            (RJSValue::Number(_), VarType::Number) => true,
            (RJSValue::String(_), VarType::String) => true,
            (RJSValue::Object(_), VarType::Object) => true,
            (RJSValue::Array(items), VarType::Array(inner)) => {
                match &**inner {
                    VarType::Any => true, // vec<any> accepts any elements
                    want => items.iter().all(|v| v.is_type(want)),
                }
            }
            // Interpret a type-literal value as matching its wrapped type.
            (RJSValue::Type(rjs_type), wanted) => rjs_type == wanted,
            (RJSValue::Undefined, VarType::Undefined) => true,
            _ => false,
        }
    }

    pub fn to_type(&self) -> VarType {
        match self {
            RJSValue::Bool(_) => VarType::Bool,
            RJSValue::Number(_) => VarType::Number,
            RJSValue::String(_) => VarType::String,
            RJSValue::Object(_) => VarType::Object,
            RJSValue::Array(items) => {
                use VarType::*;
                let mut ty: Option<VarType> = None;
                for it in items {
                    let t = it.to_type(); // convert value -> VarType
                    ty = match (ty, t) {
                        (None, newt) => Some(newt),
                        (Some(prev), newt) if prev == newt => Some(prev),
                        _ => Some(Any),
                    };
                }
                VarType::Array(Box::new(ty.unwrap_or(Any)))
            }
            RJSValue::Type(rjs_type) => rjs_type.clone(),
            RJSValue::Undefined => VarType::Undefined,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            RJSValue::String(s) => s.clone(),
            RJSValue::Number(n) => n.to_string(),
            RJSValue::Bool(b) => b.to_string(),
            RJSValue::Array(a) => format!("{:?}", a),
            RJSValue::Object(o) => format!("{:?}", o),
            RJSValue::Type(t) => format!("{:?}", t),
            RJSValue::Undefined => "undefined".into(),
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            RJSValue::String(s) => s.len() > 0,
            RJSValue::Number(n) => *n > 0.0,
            RJSValue::Bool(b) => *b,
            RJSValue::Array(a) => a.len() > 0,
            RJSValue::Object(o) => o.keys().len() > 0,
            RJSValue::Type(_) => false,
            RJSValue::Undefined => false,
        }
    }

    pub fn from_literal(lit: Literal) -> RJSValue {
        match lit {
            Literal::Number(n) => RJSValue::Number(n),
            Literal::String(s) => RJSValue::String(s),
            Literal::Bool(b) => RJSValue::Bool(b),
            Literal::Undefined => RJSValue::Undefined,
        }
    }

    pub fn string_map_to_rjs(map: &HashMap<String, String>) -> RJSValue {
        let obj: HashMap<String, RJSValue> = map
            .iter()
            .map(|(k, v)| (k.clone(), RJSValue::String(v.clone())))
            .collect();
        RJSValue::Object(obj)
    }

    pub fn rjs_to_json(value: &RJSValue) -> serde_json::Value {
        match value {
            RJSValue::Number(n) => serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(0))),
            RJSValue::Bool(b) => serde_json::Value::Bool(*b),
            RJSValue::String(s) => serde_json::Value::String(s.clone()),
            RJSValue::Array(vec) => {
                let arr = vec.iter().map(RJSValue::rjs_to_json).collect();
                serde_json::Value::Array(arr)
            }
            RJSValue::Object(map) => {
                // Convert each entry into JSON
                let mut m = serde_json::Map::new();
                for (k, v) in map {
                    m.insert(k.clone(), RJSValue::rjs_to_json(v));
                }
                serde_json::Value::Object(m)
            }
            RJSValue::Type(ty) => serde_json::Value::String(format!("{:?}", ty)),
            RJSValue::Undefined => serde_json::Value::Null,
        }
    }

    /// Convert serde_json::Value into RJSValue
    pub fn json_to_rjs(json: &JsonValue, pos: Position) -> EvalResult<RJSValue> {
        match json {
            JsonValue::Number(n) => n
                .as_f64()
                .map(RJSValue::Number)
                .ok_or_else(|| EvalError::General("Invalid number in JSON".into(), pos)),
            JsonValue::String(s) => Ok(RJSValue::String(s.clone())),
            JsonValue::Bool(b) => Ok(RJSValue::Bool(*b)),
            JsonValue::Array(arr) => {
                let mut items = Vec::with_capacity(arr.len());
                for elem in arr {
                    items.push(RJSValue::json_to_rjs(elem, pos)?);
                }
                Ok(RJSValue::Array(items))
            }
            JsonValue::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), RJSValue::json_to_rjs(v, pos)?);
                }
                Ok(RJSValue::Object(map))
            }
            _ => Ok(RJSValue::Undefined),
        }
    }

    pub fn dbvalue_to_rjs(v: &DbValue, pos: Position) -> EvalResult<RJSValue> {
        match v {
            DbValue::Number(n) => Ok(RJSValue::Number(*n)),
            DbValue::Bool(b) => Ok(RJSValue::Bool(*b)),
            DbValue::String(s) => Ok(RJSValue::String(s.clone())),
            DbValue::Null => Ok(RJSValue::Undefined),

            DbValue::Json(json) => RJSValue::json_to_rjs(json, pos),
        }
    }
}
