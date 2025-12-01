use std::fmt;

use crate::rjscript::ast::position::Position;

#[derive(Debug)]
pub enum EvalError {
    VariableNotFound(String, Position),
    VariableAlreadyDeclared(String, Position),
    FunctionAlreadyDeclared(String, Position),
    UndeclaredVariable(String, Position),
    TypeMismatch(String, Position),
    DivisionByZero(Position),
    WrongNumberOfArguments(String, u64, Position),
    General(String, Position),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::VariableNotFound(name, pos) => {
                write!(f, "Variable '{}' not found, at: {}:{}", name, pos.line, pos.column)
            },
            EvalError::VariableAlreadyDeclared(name, pos) => {
                write!(f, "Variable '{}' already declared, at: {}:{}", name, pos.line, pos.column)
            },
            EvalError::FunctionAlreadyDeclared(name, pos) => {
                write!(f, "Function '{}' already declared, at: {}:{}", name, pos.line, pos.column)
            },
            EvalError::UndeclaredVariable(name, pos) => {
                write!(f, "Variable '{}' not declared, at: {}:{}", name, pos.line, pos.column)
            },
            EvalError::TypeMismatch(field, pos) => {
                write!(f, "Type mismatch: '{}', at: {}:{}", field, pos.line, pos.column)
            },
            EvalError::DivisionByZero(pos) => write!(f, "Division by zero, at: {}:{}", pos.line, pos.column),
            EvalError::WrongNumberOfArguments(method, arg_number, pos) => write!(f, "'{}()' expects exactly {} arguments, at: {}:{}", method, arg_number, pos.line, pos.column),
            EvalError::General(msg, pos) => write!(f, "{}, at: {}:{}", msg, pos.line, pos.column),
        }
    }
}

impl std::error::Error for EvalError {}
