use crate::rjscript::{ast::position::Position, evaluator::{runtime::{eval_ctx::EvalCtx, value::RJSValue}, EvalResult}};

pub mod core;
pub mod stringcore;
pub mod arraycore;

pub type BuiltinFn = fn(ctx: &EvalCtx, Vec<RJSValue>, Position) -> EvalResult<RJSValue>;
/// Methods that do NOT mutate the receiver
pub type PureMethodFn  = fn(&RJSValue, &[RJSValue], Position) -> EvalResult<RJSValue>;
/// Methods that DO mutate the receiver
pub type MutMethodFn   = fn(&mut RJSValue, &[RJSValue], Position) -> EvalResult<RJSValue>;