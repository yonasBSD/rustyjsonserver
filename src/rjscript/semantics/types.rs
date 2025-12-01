#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VarType {
    Bool,
    Number,
    String,
    Array(Box<VarType>),
    Object,
    Any,
    Undefined
}

impl std::fmt::Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use VarType::*;
        match self {
            Bool => write!(f, "bool"),
            Number => write!(f, "num"),
            String => write!(f, "str"),
            Object => write!(f, "obj"),
            Undefined => write!(f, "undefined"),
            Any => write!(f, "any"),
            Array(inner) => write!(f, "vec<{}>", inner),
        }
    }
}