use std::collections::HashSet;

use crate::rjscript::semantics::types::VarType;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Receiver {
    Array,
    String,
}

#[derive(Debug, Clone, Copy)]
pub struct MethodMeta {
    pub name: &'static str,
    pub is_mut: bool,
    pub returns_number: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum ArrayMethod {
    Length,
    Push,
    Remove,
    RemoveAt,
}

pub const ARRAY_METHODS_META: &[(ArrayMethod, MethodMeta)] = &[
    (ArrayMethod::Length,  MethodMeta { name: "length",  is_mut: false, returns_number: true  }),
    (ArrayMethod::Push,    MethodMeta { name: "push",    is_mut: true,  returns_number: false }),
    (ArrayMethod::Remove,     MethodMeta { name: "remove",     is_mut: true,  returns_number: false }),
    (ArrayMethod::RemoveAt,   MethodMeta { name: "removeAt",   is_mut: true,  returns_number: false }),
];

#[derive(Debug, Clone, Copy)]
pub enum StringMethod {
    Length,
    Contains,
    Split,
    ToChars,
    Replace,
    Substring,
}

pub const STRING_METHODS_META: &[(StringMethod, MethodMeta)] = &[
    (StringMethod::Length,      MethodMeta { name: "length",      is_mut: false, returns_number: true  }),
    (StringMethod::Contains,    MethodMeta { name: "contains",    is_mut: false, returns_number: false }),
    (StringMethod::Split,    MethodMeta { name: "split",    is_mut: false, returns_number: false }),
    (StringMethod::ToChars,    MethodMeta { name: "to_chars",    is_mut: false, returns_number: false }),
    (StringMethod::Replace,    MethodMeta { name: "replace",    is_mut: false, returns_number: false }),
    (StringMethod::Substring,    MethodMeta { name: "substring",    is_mut: false, returns_number: false }),
];

#[inline]
pub fn receiver_from_vartype(ty: &VarType) -> Option<Receiver> {
    match ty {
        VarType::Array(_) => Some(Receiver::Array),
        VarType::String   => Some(Receiver::String),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Builtin {      
    Print,
    ToType,
    ToString,
    Sleep,
    CacheGet,
    CacheSet,
    CacheDel,
    CacheClear,
    DbCreateTable,
    DbGetAllTables,
    DbDropTable,
    DbCreateEntry,
    DbGetAll,
    DbGetById,
    DbGetByFields,
    DbUpdateById,
    DbUpdateByFields,
    DbDeleteById,
    DbDeleteByFields,
    DbDrop,
}

pub const BUILTINS_TBL: &[(Builtin, &'static str)] = &[
    (Builtin::Print,  "print"),
    (Builtin::ToType, "toType"),
    (Builtin::ToString, "toString"),
    (Builtin::Sleep, "sleep"),
    (Builtin::CacheGet, "cacheGet"),
    (Builtin::CacheSet, "cacheSet"),
    (Builtin::CacheDel, "cacheDel"),
    (Builtin::CacheClear, "cacheClear"),
    (Builtin::DbCreateTable, "dbCreateTable"),
    (Builtin::DbGetAllTables, "dbGetAllTables"),
    (Builtin::DbDropTable, "dbDropTable"),
    (Builtin::DbCreateEntry, "dbCreateEntry"),
    (Builtin::DbGetAll, "dbGetAll"),
    (Builtin::DbGetById, "dbGetById"),
    (Builtin::DbGetByFields, "dbGetByFields"),
    (Builtin::DbUpdateById, "dbUpdateById"),
    (Builtin::DbUpdateByFields, "dbUpdateByFields"),
    (Builtin::DbDeleteById, "dbDeleteById"),
    (Builtin::DbDeleteByFields, "dbDeleteByFields"),
    (Builtin::DbDrop, "dbDrop"),    
];

#[inline]
pub fn builtin_names_set() -> HashSet<&'static str> {
    BUILTINS_TBL.iter().map(|(_, n)| *n).collect()
}