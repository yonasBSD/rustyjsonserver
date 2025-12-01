pub mod engine;
pub mod runtime;
pub mod errors;
mod builtins;

pub type EvalResult<T>  = std::result::Result<T, crate::rjscript::evaluator::errors::EvalError>;