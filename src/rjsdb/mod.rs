pub mod db;

use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rjscript::evaluator::runtime::value::RJSValue;

#[derive(Clone, Serialize, Deserialize)]
pub enum DbValue {
    Number(f64),
    Bool(bool),
    String(String),
    Null,
    Json(Value),
}

impl DbValue {
    pub fn rjs_to_dbvalue(v: &RJSValue) -> DbValue {
        match v {
            RJSValue::Number(n) => DbValue::Number(*n),
            RJSValue::Bool(b) => DbValue::Bool(*b),
            RJSValue::String(s) => DbValue::String(s.clone()),

            RJSValue::Array(arr) => {
                let json_arr: Vec<Value> = arr.iter().map(|x| RJSValue::rjs_to_json(x)).collect();
                DbValue::Json(Value::Array(json_arr))
            }
            RJSValue::Object(map) => {
                let json_obj: serde_json::Map<String, Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), RJSValue::rjs_to_json(v)))
                    .collect();
                DbValue::Json(Value::Object(json_obj))
            }

            RJSValue::Type(_) | RJSValue::Undefined => DbValue::Null,
        }
    }
}

pub type FieldFilter = std::collections::BTreeMap<String, serde_json::Value>;

pub trait TableDb: Send + Sync {
    fn create_table(&self, table: &str) -> io::Result<()>;
    fn get_all_tables(&self) -> io::Result<Vec<String>>;
    fn drop_table(&self, table: &str) -> io::Result<()>;

    fn create_entry(&self, table: &str, value: DbValue) -> io::Result<String>;

    fn get_all(&self, table: &str) -> io::Result<Vec<(String, DbValue)>>;
    fn get_by_id(&self, table: &str, id: &str) -> io::Result<Option<(String, DbValue)>>;
    fn get_by_fields(
        &self,
        table: &str,
        filter: &FieldFilter,
    ) -> io::Result<Vec<(String, DbValue)>>;

    fn update_by_id(&self, table: &str, id: &str, patch: DbValue) -> io::Result<bool>;
    fn update_by_fields(
        &self,
        table: &str,
        filter: &FieldFilter,
        patch: DbValue,
    ) -> io::Result<usize>;

    fn delete_by_id(&self, table: &str, id: &str) -> io::Result<bool>;
    fn delete_by_fields(&self, table: &str, filter: &FieldFilter) -> io::Result<usize>;

    fn drop_db(&self) -> io::Result<()>;
}
