use crate::rjscript::{
        ast::{
            node::HasPos,
            stmt::{Stmt, StmtKind},
        },
        evaluator::{
            engine::controlflow::ControlFlow, errors::EvalError, runtime::{env::{Env, EnvRef, UserFunction}, eval_ctx::EvalCtx, value::RJSValue}, EvalResult
        },
    };

impl Stmt {
    pub fn eval_stmt(&self, req: &EvalCtx, env: &EnvRef) -> EvalResult<ControlFlow> {
        match &self.kind {
            StmtKind::FunctionDecl {
                ident,
                params,
                return_type,
                body,
            } => {
                let pos = self.pos();

                // Capture current env for closure
                let closure_env = Env::capture_functions_only(env);
                let func = UserFunction {
                    params: params.clone(),
                    return_type: return_type.clone(),
                    body: body.clone(),
                    env: closure_env.clone(),
                };

                // 1) Insert into *its own* closure env so recursive calls resolve:
                {
                    let mut clo = closure_env.borrow_mut();
                    clo.define_fn(ident, func.clone(), pos)?;
                }

                // 2) Also insert into the *global* env so downstream code sees it:
                {
                    let mut global = env.borrow_mut();
                    global.define_fn(ident, func.clone(), pos)?;
                }

                return Ok(ControlFlow::None(pos));
            }
            StmtKind::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                let pos = self.pos();
                let cond_val = condition.eval_expr(req, env)?;

                if cond_val.to_bool() {
                    match then_block.eval_block(req, env)? {
                        ControlFlow::None(_) => {}
                        other => return Ok(other),
                    }
                } else if let Some(else_blk) = else_block {
                    match else_blk.eval_block(req, env)? {
                        ControlFlow::None(_) => {}
                        ControlFlow::Break(_) => {} // preserve previous behavior
                        other => return Ok(other),
                    }
                }
                Ok(ControlFlow::None(pos))
            }
            StmtKind::Switch {
                condition,
                cases,
                default,
            } => {
                let pos = self.pos();
                // 1) evaluate discriminant
                let dv = condition.eval_expr(req, env)?;

                // 2) try each case in order
                let mut hit = false;
                for (case_expr, block) in cases {
                    let cv = case_expr.eval_expr(req, env)?;
                    if cv.eq(&dv) {
                        hit = true;
                        match block.eval_block(req, env)? {
                            ControlFlow::None(_) => {}
                            ControlFlow::Break(_) => {}
                            other => return Ok(other), // ignore continue inside switch
                        }
                        break;
                    }
                }

                // 3) if no case matched, run default (if any)
                if !hit {
                    if let Some(def_blk) = default {
                        match def_blk.eval_block(req, env)? {
                            ControlFlow::None(_) => {}
                            other => return Ok(other),
                        }
                    }
                }

                return Ok(ControlFlow::None(pos));
            }
            StmtKind::For {
                init,
                condition,
                increment,
                body,
            } => {
                let pos = self.pos();
                // Create a fresh child scope for the loop
                let loop_env = Env::push_scope(&env);

                // run initializer once
                if let Some(init_stmt) = init {
                    init_stmt.eval_stmt(req, &loop_env)?;
                }

                // loop
                loop {
                    // evaluate condition
                    let cond_v = condition.eval_expr(req, &loop_env)?;
                    if !cond_v.to_bool() {
                        break;
                    }

                    let body_env = Env::push_scope(&loop_env);

                    // body
                    match body.eval_block(req, &body_env)? {
                        ControlFlow::Break(_) => break,
                        ControlFlow::Continue(_) => {
                            if let Some(inc_e) = increment {
                                inc_e.eval_expr(req, &loop_env)?;
                            }
                            continue;
                        }
                        ControlFlow::Return(v, pos) => return Ok(ControlFlow::Return(v, pos)),
                        ControlFlow::ReturnStatus(code, v, pos) => {
                            return Ok(ControlFlow::ReturnStatus(code, v, pos))
                        }
                        ControlFlow::None(_) => {}
                    }

                    // increment
                    if let Some(inc_expr) = increment {
                        // ignore its value; side-effects only
                        inc_expr.eval_expr(req, &loop_env)?;
                    }
                }
                Ok(ControlFlow::None(pos))
            }
            StmtKind::Break => return Ok(ControlFlow::Break(self.pos())),
            StmtKind::Continue => return Ok(ControlFlow::Continue(self.pos())),
            StmtKind::Return(expr) => {
                let val = expr.eval_expr(req, env)?;
                return Ok(ControlFlow::Return(val, self.pos()));
            }
            StmtKind::ReturnStatus{status, value} => {
                let pos = self.pos();
                let status_val = status.eval_expr(req, env)?;
                let status_num = match status_val {
                    RJSValue::Number(n) => n as u16,
                    other => {
                        return Err(EvalError::TypeMismatch(
                            format!("Status code must be a number, got {:?}", other),
                            pos,
                        ));
                    }
                };
                let response_val = value.eval_expr(req, env)?;

                return Ok(ControlFlow::ReturnStatus(status_num, response_val, pos));
            }
            StmtKind::Let{name, ty, init} => {
                let pos = self.pos();
                // Evaluate or default to Undefined
                let val = if let Some(expr) = init {
                    let v = expr.eval_expr(req, env)?;
                    if !v.is_type(&ty) {
                        return Err(EvalError::General(
                            format!(
                                "Type mismatch initializing {}: expected {:?}, got {:?}",
                                name, ty, v
                            ),
                            pos,
                        ));
                    }
                    v
                } else {
                    // no initializer → undefined
                    RJSValue::Undefined
                };
                env.borrow_mut()
                    .declare_var(&name.clone(), ty.clone(), val, pos)?;
                Ok(ControlFlow::None(pos))
            }
            StmtKind::ExprStmt(expr) => {
                expr.eval_expr(req, env)?;
                Ok(ControlFlow::None(self.pos()))
            }
        }
    }
}
