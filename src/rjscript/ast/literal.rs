use crate::rjscript::semantics::types::VarType;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Bool(bool),
    Undefined,
}

impl Literal {
    pub fn to_type(&self) -> VarType {
        match self {
            Literal::Number(_) => VarType::Number,
            Literal::String(_) => VarType::String,
            Literal::Bool(_) => VarType::Bool,
            Literal::Undefined => VarType::Undefined,
        }
    }
}