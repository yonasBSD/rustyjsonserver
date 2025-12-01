use crate::rjscript::{ast::position::Position, evaluator::runtime::value::RJSValue};

pub enum ControlFlow {
    None(Position),
    Break(Position),
    Continue(Position),
    Return(RJSValue, Position),
    ReturnStatus(u16, RJSValue, Position),
}