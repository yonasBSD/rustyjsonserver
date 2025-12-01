use crate::rjscript::{
    ast::{binop::BinOp, position::Position},
    evaluator::{errors::EvalError, runtime::value::RJSValue, EvalResult},
};

impl BinOp {
    pub fn eval_binop(&self, lv: &RJSValue, rv: &RJSValue, pos: Position) -> EvalResult<RJSValue> {
        match self {
            BinOp::Add => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Number(a + b)),
                (RJSValue::String(a), RJSValue::String(b)) => {
                    let mut s = a.clone();
                    s.push_str(b);
                    Ok(RJSValue::String(s))
                }
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot add {:?} + {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Sub => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Number(a - b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot subtract {:?} - {:?}", lv, rv),
                    pos,
                )),
            },

            BinOp::Mul => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Number(a * b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot multiply {:?} * {:?}", lv, rv),
                    pos,
                )),
            },

            BinOp::Div => match (&lv, &rv) {
                (RJSValue::Number(_), RJSValue::Number(0.0)) => Err(EvalError::DivisionByZero(pos)),
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Number(a / b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot divide {:?} / {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Rem => match (&lv, &rv) {
                (RJSValue::Number(_), RJSValue::Number(0.0)) => Err(EvalError::DivisionByZero(pos)),
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Number(a % b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot modulo {:?} % {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Eq => match (&lv, &rv) {
                _ => Ok(RJSValue::Bool(lv == rv)),
            },
            BinOp::Ne => Ok(RJSValue::Bool(lv != rv)),
            BinOp::Lt => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Bool(a < b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot compare {:?} < {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Le => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Bool(a <= b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot compare {:?} <= {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Gt => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Bool(a > b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot compare {:?} > {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Ge => match (&lv, &rv) {
                (RJSValue::Number(a), RJSValue::Number(b)) => Ok(RJSValue::Bool(a >= b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot compare {:?} >= {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::And => match (&lv, &rv) {
                (RJSValue::Bool(a), RJSValue::Bool(b)) => Ok(RJSValue::Bool(*a && *b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot apply && to {:?} and {:?}", lv, rv),
                    pos,
                )),
            },
            BinOp::Or => match (&lv, &rv) {
                (RJSValue::Bool(a), RJSValue::Bool(b)) => Ok(RJSValue::Bool(*a || *b)),
                _ => Err(EvalError::TypeMismatch(
                    format!("Cannot apply || to {:?} and {:?}", lv, rv),
                    pos,
                )),
            },
        }
    }
}
