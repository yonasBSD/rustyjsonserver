use crate::rjscript::{
        ast::{block::Block, node::HasPos, position::Position},
        evaluator::{engine::controlflow::ControlFlow, runtime::{env::EnvRef, eval_ctx::EvalCtx}, EvalResult},
    };

impl Block {
    /// Evaluate each statement in order.
    /// Return `Ok(Some(v))` on `return`, or `Ok(None)` if none.
    pub fn eval_block(&self, req: &EvalCtx, env: &EnvRef) -> EvalResult<ControlFlow> {
        let mut last_pos: Option<Position> = None;
        for stmt in &self.stmts {
            match stmt.eval_stmt(req, env)? {
                ControlFlow::None(pos) => {
                    last_pos = Some(pos);
                    continue;
                }
                other => return Ok(other),
            }
        }
        
        match last_pos {
            Some(pos) => Ok(ControlFlow::None(pos)),
            None => Ok(ControlFlow::None(self.pos()))
        }
    }
}
