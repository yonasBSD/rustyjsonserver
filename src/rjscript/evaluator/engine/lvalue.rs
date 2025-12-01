use crate::rjscript::ast::expr::ExprKind;
use crate::rjscript::ast::node::HasPos;
use crate::rjscript::ast::{expr::Expr, position::Position};
use crate::rjscript::evaluator::runtime::env::EnvRef;
use crate::rjscript::evaluator::runtime::eval_ctx::EvalCtx;
use crate::rjscript::evaluator::runtime::value::RJSValue;
use crate::rjscript::evaluator::{errors::EvalError, EvalResult};

#[derive(Debug, Clone)]
pub enum LhsStep {
    Field(String),
    Index(usize),
}

/// Resolve an expression into (root variable name, path of steps) for assignment/mutation.
/// Fails if target is not a variable/member/index chain, or if it comes from req.* (immutable).
pub fn resolve_var_and_path(
    target: &Expr,
    ctx: &EvalCtx,
    env: &EnvRef,
) -> EvalResult<(String, Vec<LhsStep>)> {
    // 1) Block request-derived assignments up-front (no alias tracking here by design).
    if target.is_request_derived() {
        return Err(EvalError::General("Request fields are immutable".into(), target.pos()));
    }

    // 2) We need the root variable name (e.g., "user" in user.a[0].b)
    let Some(root) = Expr::root_ident(target) else {
        return Err(EvalError::General(
            "Invalid assignment target (not a variable, member, or index)".into(),
            target.pos(),
        ));
    };

    // 3) Walk from leaf to root, collecting steps
    let mut steps: Vec<LhsStep> = Vec::new();
    let mut cur = target;

    loop {
        match &cur.kind {
            ExprKind::Member { object, property } => {
                steps.push(LhsStep::Field(property.clone()));
                cur = object;
            }
            ExprKind::Index { object, index } => {
                let iv = index.eval_expr(ctx, env)?;
                match iv {
                    RJSValue::Number(n) => {
                        if !n.is_finite() || n.fract() != 0.0 || n < 0.0 {
                            return Err(EvalError::General(
                                format!("Index must be a non-negative integer, got {}", n),
                                index.pos(),
                            ));
                        }
                        steps.push(LhsStep::Index(n as usize));
                    }
                    RJSValue::String(s) => steps.push(LhsStep::Field(s)),
                    other => {
                        return Err(EvalError::TypeMismatch(
                            format!("Index must be number or string, got {:?}", other),
                            index.pos(),
                        ));
                    }
                }
                cur = object;
            }
            ExprKind::Ident(_) => {
                break;
            }
            _ => {
                return Err(EvalError::General(
                    "Invalid assignment target (not a variable, member, or index)".into(),
                    cur.pos(),
                ));
            }
        }
    }

    steps.reverse(); // root -> ... -> leaf
    Ok((root.to_string(), steps))
}

/// Walk the path on a mutable value and return a mutable reference to the destination slot.
pub fn navigate_mut_slot<'a>(
    root: &'a mut RJSValue,
    path: &[LhsStep],
    pos: Position,
) -> EvalResult<&'a mut RJSValue> {
    let mut cur = root;
    for step in path {
        match (step, cur) {
            (LhsStep::Field(k), RJSValue::Object(map)) => {
                cur = map.get_mut(k).ok_or_else(|| {
                    EvalError::General(format!("Property '{}' not found", k), pos)
                })?;
            }
            (LhsStep::Index(i), RJSValue::Array(vec)) => {
                if *i >= vec.len() {
                    return Err(EvalError::General(
                        format!("Index {} out of bounds (len={})", i, vec.len()),
                        pos,
                    ));
                }
                cur = &mut vec[*i];
            }
            (LhsStep::Field(_), _) => {
                return Err(EvalError::TypeMismatch(
                    "Tried to access field on non-object".into(),
                    pos,
                ));
            }
            (LhsStep::Index(_), _) => {
                return Err(EvalError::TypeMismatch(
                    "Tried to index into non-array".into(),
                    pos,
                ));
            }
        }
    }
    Ok(cur)
}
