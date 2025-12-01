use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    thread,
    time::Duration,
};

use crate::{
    rjscript::{
        ast::position::Position,
        evaluator::{
            builtins::BuiltinFn,
            errors::EvalError,
            runtime::{eval_ctx::EvalCtx, value::RJSValue},
            EvalResult,
        },
        semantics::methods::{Builtin, BUILTINS_TBL},
    },
    rjsdb::DbValue,
};

static BUILTINS: OnceLock<Arc<HashMap<String, BuiltinFn>>> = OnceLock::new();

fn builtin_impl(b: Builtin) -> BuiltinFn {
    match b {
        Builtin::Print => builtin_print,
        Builtin::ToType => builtin_to_type,
        Builtin::ToString => builtin_to_string,
        Builtin::Sleep => builtin_sleep,
        Builtin::CacheGet => builtin_cache_get,
        Builtin::CacheSet => builtin_cache_set,
        Builtin::CacheDel => builtin_cache_del,
        Builtin::CacheClear => builtin_cache_clear,
        Builtin::DbCreateTable => db_create_table,
        Builtin::DbGetAllTables => db_get_all_tables,
        Builtin::DbDropTable => db_drop_table,
        Builtin::DbCreateEntry => db_create_entry,
        Builtin::DbGetAll => db_get_all,
        Builtin::DbGetById => db_get_by_id,
        Builtin::DbGetByFields => db_get_by_fields,
        Builtin::DbUpdateById => db_update_by_id,
        Builtin::DbUpdateByFields => db_update_by_fields,
        Builtin::DbDeleteById => db_delete_by_id,
        Builtin::DbDeleteByFields => db_delete_by_fields,
        Builtin::DbDrop => db_drop,
    }
}

pub fn builtins_table() -> Arc<HashMap<String, BuiltinFn>> {
    BUILTINS
        .get_or_init(|| {
            let mut m = HashMap::new();
            for (k, name) in BUILTINS_TBL {
                m.insert((*name).to_string(), builtin_impl(*k));
            }
            Arc::new(m)
        })
        .clone()
}

fn builtin_print(_: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() < 1 {
        return Err(EvalError::General(
            "'print' expects at least 1 argument".into(),
            pos,
        ));
    }

    let out = args
        .into_iter()
        .map(|v| match v {
            RJSValue::String(s) => s,
            RJSValue::Number(n) => n.to_string(),
            RJSValue::Bool(b) => b.to_string(),
            RJSValue::Array(a) => format!("{:?}", a),
            RJSValue::Object(o) => format!("{:?}", o),
            RJSValue::Type(ty) => format!("{:?}", ty),
            RJSValue::Undefined => format!("undefined"),
        })
        .collect::<Vec<_>>()
        .join("");
    println!("{}", out);
    Ok(RJSValue::Bool(true))
}

fn builtin_to_string(_: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() < 1 {
        return Err(EvalError::WrongNumberOfArguments("toString".into(), 1, pos));
    }

    let value = args.into_iter().next().unwrap();
    Ok(RJSValue::String(value.to_string()))
}

fn builtin_to_type(_: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("toType".into(), 1, pos));
    }
    let value = args.into_iter().next().unwrap();
    Ok(RJSValue::Type(value.to_type()))
}

fn builtin_sleep(_: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("sleep".into(), 1, pos));
    }
    match &args[0] {
        RJSValue::Number(ms) => {
            // interpret as milliseconds
            thread::sleep(Duration::from_millis(*ms as u64));
            Ok(RJSValue::Bool(true))
        }
        other => Err(EvalError::TypeMismatch(
            format!("sleep() expects a number, got {:?}", other),
            pos,
        )),
    }
}

pub fn builtin_cache_get(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("cacheGet".into(), 1, pos));
    }
    if let RJSValue::String(key) = &args[0] {
        Ok(ctx.globals.cache.get(&key).unwrap_or(RJSValue::Undefined))
    } else {
        Err(EvalError::TypeMismatch(
            "cacheGet needs a string key".into(),
            pos,
        ))
    }
}

pub fn builtin_cache_set(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    // cacheSet(key, value, ttlSeconds?)
    if args.len() < 2 || args.len() > 3 {
        return Err(EvalError::WrongNumberOfArguments("cacheSet".into(), 2, pos));
    }
    if let RJSValue::String(key) = &args[0] {
        let value = args[1].clone();

        ctx.globals.cache.set(key.clone(), value);
        Ok(RJSValue::Undefined)
    } else {
        Err(EvalError::TypeMismatch(
            "cacheSet needs a string key".into(),
            pos,
        ))
    }
}

pub fn builtin_cache_del(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("cacheDel".into(), 1, pos));
    }
    if let RJSValue::String(key) = &args[0] {
        Ok(RJSValue::Bool(ctx.globals.cache.del(&key)))
    } else {
        Err(EvalError::TypeMismatch(
            "cacheDel needs a string key".into(),
            pos,
        ))
    }
}

pub fn builtin_cache_clear(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 0 {
        return Err(EvalError::WrongNumberOfArguments(
            "cacheClear".into(),
            0,
            pos,
        ));
    }
    ctx.globals.cache.clear();
    Ok(RJSValue::Bool(true))
}

pub fn db_create_table(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbCreateTable".into(),
            1,
            pos,
        ));
    }

    let name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    match ctx.globals.db.as_ref() {
        Some(db) => {
            db.create_table(&name)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Undefined)
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_get_all_tables(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 0 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbGetAllTables".into(),
            0,
            pos,
        ));
    }

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let tables = db
                .get_all_tables()
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            let rjs_tables = tables
                .into_iter()
                .map(RJSValue::String)
                .collect::<Vec<RJSValue>>();
            Ok(RJSValue::Array(rjs_tables))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_drop_table(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbDropTable".into(),
            1,
            pos,
        ));
    }

    let name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    match ctx.globals.db.as_ref() {
        Some(db) => {
            db.drop_table(&name)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Undefined)
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_create_entry(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbCreateEntry".into(),
            2,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let entry = &args[1];

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let rjs_to_dbvalue = DbValue::rjs_to_dbvalue(entry);
            let id = db.create_entry(&table_name, rjs_to_dbvalue)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::String(id))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_get_all(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 1 {
        return Err(EvalError::WrongNumberOfArguments("dbGetAll".into(), 1, pos));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let entries = db
                .get_all(&table_name)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            // Transform each DB entry into an object that merges the stored object (if any)
            // with the top-level "id" field. Non-object primitive values are wrapped under "value".
            let rjs_entries = entries
                .into_iter()
                .map(|(id, value)| {
                    let converted = match value {
                        DbValue::Bool(b) => RJSValue::Bool(b),
                        DbValue::Number(n) => RJSValue::Number(n),
                        DbValue::String(s) => RJSValue::String(s),
                        DbValue::Json(j) => match RJSValue::json_to_rjs(&j, pos) {
                            Ok(v) => v,
                            Err(_) => RJSValue::Undefined,
                        },
                        DbValue::Null => RJSValue::Undefined,
                    };
                    match converted {
                        RJSValue::Object(mut obj) => {
                            // Insert/override id field
                            obj.insert("id".to_string(), RJSValue::String(id));
                            RJSValue::Object(obj)
                        }
                        other => {
                            let mut obj = HashMap::new();
                            obj.insert("id".to_string(), RJSValue::String(id));
                            obj.insert("value".to_string(), other);
                            RJSValue::Object(obj)
                        }
                    }
                })
                .collect();
            Ok(RJSValue::Array(rjs_entries))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_get_by_id(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbGetById".into(),
            2,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let id = match &args[1] {
        RJSValue::String(s) => s.clone(),
        _ => return Err(EvalError::TypeMismatch("id must be string".into(), pos)),
    };

    match ctx.globals.db.as_ref() {
        Some(db) => {
            match db
                .get_by_id(&table_name, &id)
                .map_err(|e| EvalError::General(e.to_string(), pos))?
            {
                Some((id, value)) => {
                    let converted = match value {
                        DbValue::Bool(b) => RJSValue::Bool(b),
                        DbValue::Number(n) => RJSValue::Number(n),
                        DbValue::String(s) => RJSValue::String(s),
                        DbValue::Json(j) => match RJSValue::json_to_rjs(&j, pos) {
                            Ok(v) => v,
                            Err(_) => RJSValue::Undefined,
                        },
                        DbValue::Null => RJSValue::Undefined,
                    };
                    match converted {
                        RJSValue::Object(mut obj) => {
                            // Insert/override id field
                            obj.insert("id".to_string(), RJSValue::String(id));
                            Ok(RJSValue::Object(obj))
                        }
                        other => {
                            let mut obj = HashMap::new();
                            obj.insert("id".to_string(), RJSValue::String(id));
                            obj.insert("value".to_string(), other);
                            Ok(RJSValue::Object(obj))
                        }
                    }
                }
                None => Ok(RJSValue::Undefined),
            }
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_get_by_fields(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbGetByFields".into(),
            2,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let field_filter = match &args[1] {
        RJSValue::Object(o) => {
            let mut filter = std::collections::BTreeMap::new();
            for (k, v) in o.iter() {
                filter.insert(k.clone(), RJSValue::rjs_to_json(v));
            }
            filter
        }
        _ => {
            return Err(EvalError::TypeMismatch(
                "field filter must be an object".into(),
                pos,
            ))
        }
    };
    
    match ctx.globals.db.as_ref() {
        Some(db) => {
            let entries = db
                .get_by_fields(&table_name, &field_filter)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            let rjs_entries = entries
                .into_iter()
                .map(|(id, value)| {
                    let converted = match value {
                        DbValue::Bool(b) => RJSValue::Bool(b),
                        DbValue::Number(n) => RJSValue::Number(n),
                        DbValue::String(s) => RJSValue::String(s),
                        DbValue::Json(j) => match RJSValue::json_to_rjs(&j, pos) {
                            Ok(v) => v,
                            Err(_) => RJSValue::Undefined,
                        },
                        DbValue::Null => RJSValue::Undefined,
                    };
                    match converted {
                        RJSValue::Object(mut obj) => {
                            obj.insert("id".to_string(), RJSValue::String(id));
                            RJSValue::Object(obj)
                        }
                        other => {
                            let mut obj = HashMap::new();
                            obj.insert("id".to_string(), RJSValue::String(id));
                            obj.insert("value".to_string(), other);
                            RJSValue::Object(obj)
                        }
                    }
                })
                .collect();
            Ok(RJSValue::Array(rjs_entries))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_update_by_id(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 3 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbUpdateById".into(),
            3,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let id = match &args[1] {
        RJSValue::String(s) => s.clone(),
        _ => return Err(EvalError::TypeMismatch("id must be string".into(), pos)),
    };

    let patch = &args[2];

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let updated = db
                .update_by_id(&table_name, &id, DbValue::rjs_to_dbvalue(patch))
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Bool(updated))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_update_by_fields(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 3 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbUpdateByFields".into(),
            3,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let field_filter = match &args[1] {
        RJSValue::Object(o) => {
            let mut filter = std::collections::BTreeMap::new();
            for (k, v) in o.iter() {
                filter.insert(k.clone(), RJSValue::rjs_to_json(v));
            }
            filter
        }
        _ => {
            return Err(EvalError::TypeMismatch(
                "field filter must be an object".into(),
                pos,
            ))
        }
    };

    let patch = &args[2];

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let updated_count = db
                .update_by_fields(&table_name, &field_filter, DbValue::rjs_to_dbvalue(patch))
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Number(updated_count as f64))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_delete_by_id(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbDeleteById".into(),
            2,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let id = match &args[1] {
        RJSValue::String(s) => s.clone(),
        _ => return Err(EvalError::TypeMismatch("id must be string".into(), pos)),
    };

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let deleted = db
                .delete_by_id(&table_name, &id)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Bool(deleted))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_delete_by_fields(
    ctx: &EvalCtx,
    args: Vec<RJSValue>,
    pos: Position,
) -> EvalResult<RJSValue> {
    if args.len() != 2 {
        return Err(EvalError::WrongNumberOfArguments(
            "dbDeleteByFields".into(),
            2,
            pos,
        ));
    }

    let table_name = match &args[0] {
        RJSValue::String(s) => s.clone(),
        _ => {
            return Err(EvalError::TypeMismatch(
                "table name must be string".into(),
                pos,
            ))
        }
    };

    let field_filter = match &args[1] {
        RJSValue::Object(o) => {
            let mut filter = std::collections::BTreeMap::new();
            for (k, v) in o.iter() {
                filter.insert(k.clone(), RJSValue::rjs_to_json(v));
            }
            filter
        }
        _ => {
            return Err(EvalError::TypeMismatch(
                "field filter must be an object".into(),
                pos,
            ))
        }
    };

    match ctx.globals.db.as_ref() {
        Some(db) => {
            let deleted_count = db
                .delete_by_fields(&table_name, &field_filter)
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Number(deleted_count as f64))
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}

pub fn db_drop(ctx: &EvalCtx, args: Vec<RJSValue>, pos: Position) -> EvalResult<RJSValue> {
    if args.len() != 0 {
        return Err(EvalError::WrongNumberOfArguments("dbDrop".into(), 0, pos));
    }

    match ctx.globals.db.as_ref() {
        Some(db) => {
            db.drop_db()
                .map_err(|e| EvalError::General(e.to_string(), pos))?;
            Ok(RJSValue::Undefined)
        }
        None => Err(EvalError::General(
            "Persistent DB not configured (set RJS_DB_DIR)".into(),
            pos,
        )),
    }
}