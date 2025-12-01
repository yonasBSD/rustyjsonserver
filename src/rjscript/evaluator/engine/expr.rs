use std::{collections::HashMap, rc::Rc};


use crate::rjscript::{
        ast::{
            expr::{Expr, ExprKind, TemplatePart},
            node::HasPos,
            request::RequestFieldType,
        },
        evaluator::{
            engine::{controlflow::ControlFlow, lvalue::{navigate_mut_slot, resolve_var_and_path, LhsStep}}, errors::EvalError, runtime::{env::{Env, EnvRef}, eval_ctx::EvalCtx, runtime_globals::MethodImpl, value::RJSValue}, EvalResult
        },
    };

impl Expr {
    pub fn eval_expr(&self, ctx: &EvalCtx, env: &EnvRef) -> EvalResult<RJSValue> {
        match &self.kind {
            ExprKind::Literal(lit) => Ok(RJSValue::from_literal(lit.clone())),

            ExprKind::Template(parts) => {
                // Evaluate each part and build a single string
                let mut out = String::new();
                for part in parts {
                    match part {
                        TemplatePart::Text(t) => out.push_str(t),
                        TemplatePart::Expr(e) => {
                            let v = e.eval_expr(ctx, env)?;
                            out.push_str(&v.to_string());
                        }
                    }
                }
                Ok(RJSValue::String(out))
            }

            ExprKind::TypeLiteral(ty) => Ok(RJSValue::Type(ty.clone())),

            ExprKind::ObjectLiteral { fields } => {
                let mut map = HashMap::new();
                for (key, expr) in fields {
                    let v = expr.eval_expr(ctx, env)?;
                    map.insert(key.clone(), v);
                }
                Ok(RJSValue::Object(map))
            }

            ExprKind::RequestField(field_type) => {
                Ok(match field_type {
                    RequestFieldType::BodyField => ctx.req.body(),
                    RequestFieldType::ParamField => ctx.req.route_params(),
                    RequestFieldType::QueryField => ctx.req.query_params(),
                    RequestFieldType::HeadersField => ctx.req.headers(),
                })
            }

            ExprKind::Ident(var_name) => {
                if let Some((_, val)) = env.borrow().get_var(var_name) {
                    Ok(val)
                } else {
                    Err(EvalError::VariableNotFound(var_name.clone(), self.pos()))
                }
            }

            ExprKind::BinaryOp { op, left, right } => {
                let pos = self.pos();

                match op {
                    // Short-circuit AND: if left is false, return false without evaluating right
                    crate::rjscript::ast::binop::BinOp::And => {
                        let lv = left.eval_expr(ctx, env)?;
                        if !lv.to_bool() {
                            return Ok(RJSValue::Bool(false));
                        }
                        let rv = right.eval_expr(ctx, env)?;
                        Ok(RJSValue::Bool(rv.to_bool()))
                    }

                    // Short-circuit OR: if left is true, return true without evaluating right
                    crate::rjscript::ast::binop::BinOp::Or => {
                        let lv = left.eval_expr(ctx, env)?;
                        if lv.to_bool() {
                            return Ok(RJSValue::Bool(true));
                        }
                        let rv = right.eval_expr(ctx, env)?;
                        Ok(RJSValue::Bool(rv.to_bool()))
                    }

                    // All other binary ops keep the existing eager evaluation
                    _ => {
                        let lv = left.eval_expr(ctx, env)?;
                        let rv = right.eval_expr(ctx, env)?;
                        op.eval_binop(&lv, &rv, pos)
                    }
                }
            }

            // Array literal: evaluate all elements into a Vec<RJSValue>
            ExprKind::Array(elements) => {
                let mut vals = Vec::with_capacity(elements.len());
                for elt in elements {
                    let v = elt.eval_expr(ctx, env)?;
                    vals.push(v);
                }
                Ok(RJSValue::Array(vals))
            }

            ExprKind::AssignVar { name, value } => {
                // evaluate right-hand side
                let v = value.eval_expr(ctx, env)?;
                Ok(env.borrow_mut().assign_var(name, v, self.pos())?)
            }

            ExprKind::AssignMember {
                object,
                property,
                value,
            } => {
                let pos = self.pos();
                let (root, mut path) = resolve_var_and_path(object, ctx, env)?;
                path.push(LhsStep::Field(property.clone()));
                let v = value.eval_expr(ctx, env)?;
                let env_ref = Rc::clone(env);
                Env::with_var_slot(&env_ref, &root, |_decl_ty, root_slot| {
                    let slot = navigate_mut_slot(root_slot, &path, pos)?;
                    *slot = v.clone();
                    Ok(v.clone())
                })
                .ok_or_else(|| EvalError::VariableNotFound(root, pos))?
            }

            ExprKind::AssignIndex {
                object,
                index,
                value,
            } => {
                // turn the index expr into a usize step and append
                let pos = self.pos();
                let (root, mut path) = resolve_var_and_path(object, ctx, env)?;
                let idx_val = index.eval_expr(ctx, env)?;
                let idx = match idx_val {
                    RJSValue::Number(n) if n >= 0.0 => n as usize,
                    RJSValue::Number(n) => {
                        return Err(EvalError::General(format!("Negative index {}", n), pos))
                    }
                    other => {
                        return Err(EvalError::TypeMismatch(
                            format!("Index must be number, got {:?}", other),
                            pos,
                        ))
                    }
                };
                path.push(LhsStep::Index(idx));

                let v = value.eval_expr(ctx, env)?;
                let env_ref = Rc::clone(env);
                Env::with_var_slot(&env_ref, &root, |_decl_ty, root_slot| {
                    let slot = navigate_mut_slot(root_slot, &path, pos)?;
                    *slot = v.clone();
                    Ok(v.clone())
                })
                .ok_or_else(|| EvalError::VariableNotFound(root, pos))?
            }

            // Indexing: first eval object, then index, then bound-check
            ExprKind::Index { object, index } => {
                let pos = self.pos();
                let obj_val = object.eval_expr(ctx, env)?;
                let idx_val = index.eval_expr(ctx, env)?;

                // Ensure index is a non-negative number
                match idx_val {
                    RJSValue::Number(idx) => {
                        if idx >= 0.0 {
                            // Ensure we’re indexing into an array
                            if let RJSValue::Array(arr) = obj_val {
                                let arr_index = idx as usize;
                                if arr_index < arr.len() {
                                    Ok(arr[arr_index].clone())
                                } else {
                                    Err(EvalError::General(
                                        format!("Index {} out of bounds (len={})", idx, arr.len()),
                                        pos,
                                    ))
                                }
                            } else {
                                Err(EvalError::TypeMismatch(
                                    format!(
                                        "Cannot index into non-array or non-object value {:?}",
                                        obj_val
                                    ),
                                    pos,
                                ))
                            }
                        } else {
                            return Err(EvalError::General(format!("Negative index {}", idx), pos));
                        }
                    }
                    RJSValue::String(key) => {
                        // Ensure we’re indexing into an array
                        if let RJSValue::Object(obj) = obj_val {
                            if let Some(value) = obj.get(&key) {
                                Ok(value.clone())
                            } else {
                                Err(EvalError::General(
                                    format!("Object doesn't contain key: {}", key),
                                    pos,
                                ))
                            }
                        } else {
                            Err(EvalError::TypeMismatch(
                                format!("Cannot search for key on non-object value {:?}", obj_val),
                                pos,
                            ))
                        }
                    }
                    other => {
                        return Err(EvalError::TypeMismatch(
                            format!("Array index must be a number, got {:?}", other),
                            pos,
                        ));
                    }
                }
            }

            // Property access: obj.prop
            ExprKind::Member { object, property } => {
                let pos = self.pos();
                let obj_val = object.eval_expr(ctx, env)?;
                if let RJSValue::Object(map) = obj_val {
                    Ok(map
                        .get(property)
                        .cloned()
                        .unwrap_or(RJSValue::Undefined))
                } else {
                    Err(EvalError::General(
                        format!("Cannot read property '{}' of non-object value", property),
                        pos,
                    ))
                }
            }

            ExprKind::Call { callee, args } => {
                let pos = self.pos();
                if let ExprKind::Member { object, property } = &callee.kind {
                    // Evaluate receiver value and its type
                    let obj_val = object.eval_expr(ctx, env)?;
                    let recv_ty = obj_val.to_type();
                    // Evaluate arguments now
                    let arg_vals = args
                        .iter()
                        .map(|e| e.eval_expr(ctx, env))
                        .collect::<Result<Vec<_>, _>>()?;

                    // 2) resolve mut method in a short scope to drop the Ref<Env> immediately
                    let mut_impl = ctx.globals.resolve_method(&recv_ty, property, /* wants_mut */ true);

                    // Mutating methods first (only on owned variables, not request-derived)
                    if let Some(MethodImpl::Mut(f)) = mut_impl {
                        if object.is_request_derived() {
                            return Err(EvalError::General(
                                format!(
                                    "Cannot call mutating method '{}' on request fields",
                                    property
                                ),
                                pos,
                            ));
                        }

                        let (root, path) = resolve_var_and_path(object, ctx, env)?;
                        let env_ref = Rc::clone(env);

                        return Env::with_var_slot(&env_ref, &root, |_decl_ty, root_slot| {
                            let target = navigate_mut_slot(root_slot, &path, pos)?;
                            f(target, &arg_vals, pos)
                        })
                        .ok_or_else(|| EvalError::VariableNotFound(root.clone(), pos))?;
                    }

                    // 3) same for pure methods
                    let pure_impl = ctx.globals.resolve_method(&recv_ty, property, /* wants_mut */ false);

                    if let Some(MethodImpl::Pure(f)) = pure_impl {
                        return f(&obj_val, &arg_vals, pos);
                    }
                }

                if let ExprKind::Ident(ref name) = callee.kind {
                    // 1) built-in?
                    if let Some(builtin) = ctx.globals.get_builtin(name).cloned() {
                        // evaluate args…
                        let evaluated = args
                            .iter()
                            .map(|e| e.eval_expr(ctx, env))
                            .collect::<Result<Vec<_>, _>>()?;
                        return builtin(ctx, evaluated, pos);
                    }

                    // Then check user-defined functions
                    if let Some(func) = env.borrow().get_fn(name) {
                        if func.params.len() != args.len() {
                            return Err(EvalError::General(
                                format!(
                                    "Expected {} args but got {}",
                                    func.params.len(),
                                    args.len()
                                ),
                                pos,
                            ));
                        }
                        // 1) Evaluate all args into owned values (env borrow ends after collect)
                        let arg_vals: Vec<RJSValue> = args
                            .iter()
                            .map(|arg| arg.eval_expr(ctx, env))
                            .collect::<Result<_, EvalError>>()?;

                        // 2) Now bind them into the new call_env
                        let call_env = Env::push_scope(&func.env);
                        for ((param_name, param_type), arg_val) in func.params.iter().zip(arg_vals)
                        {
                            if !arg_val.is_type(param_type) {
                                return Err(EvalError::General(
                                    format!(
                                        "Type mismatch for {}: expected {:?}, got {:?}",
                                        param_name, param_type, arg_val
                                    ),
                                    pos,
                                ));
                            }
                            call_env.borrow_mut().declare_var(
                                param_name,
                                param_type.clone(),
                                arg_val,
                                pos,
                            )?;
                        }

                        match func.body.eval_block(ctx, &call_env)? {
                            ControlFlow::Return(v, pos) => {
                                if !v.is_type(&func.return_type) {
                                    return Err(EvalError::General(
                                        format!("Function '{}' returned type mismatch: expected {:?}, got {:?}",
                                        name, func.return_type, v), pos));
                                }
                                return Ok(v);
                            }
                            ControlFlow::ReturnStatus(_, _, pos) => {
                                // <- forbid status-returns here
                                return Err(EvalError::General(
                                    "`return status, body` is only allowed at the top level".into(),
                                    pos,
                                ));
                            }
                            ControlFlow::Break(pos) | ControlFlow::Continue(pos) => {
                                return Err(EvalError::General(
                                    "`break`/`continue` not allowed inside functions".into(),
                                    pos,
                                ));
                            }
                            ControlFlow::None(pos) => {
                                return Err(EvalError::General(
                                    format!("Function '{}' missing return value", name),
                                    pos,
                                ));
                            }
                        }
                    }
                }
                Err(EvalError::General(
                    format!("Unknown function call: {:?}", callee),
                    pos,
                ))
            }
        }
    }
}
