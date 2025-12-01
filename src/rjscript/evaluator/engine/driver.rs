use std::sync::Arc;

use crate::{http::request::Request, rjscript::{ast::block::Block, evaluator::{engine::controlflow::ControlFlow, errors::EvalError, runtime::{env::Env, eval_ctx::EvalCtx, request_cache::RequestCache, runtime_globals::RuntimeGlobals, value::RJSValue}, EvalResult}}};

/// Evaluate top-level script
pub fn eval_script(block: &Block, req: &Request) -> EvalResult<(u16, RJSValue)> {
    let globals = RuntimeGlobals::get();

    // Per-request ctx
    let req_ctx = Arc::new(RequestCache::from_request(req.clone())?);
    let ctx = EvalCtx::new(globals, req_ctx);

    let env = Env::new_ref();

    match block.eval_block(&ctx, &env)? {
        ControlFlow::ReturnStatus(code, v, _) => Ok((code, v)),

        ControlFlow::Return(v, _) => Ok((200, v)),

        ControlFlow::None(pos) => Err(EvalError::General(
            "Script must return a status code and a value, no return found".into(),
            pos,
        )),

        ControlFlow::Break(pos) => Err(EvalError::General(
            "Unexpected `break` at top level".into(),
            pos,
        )),
        ControlFlow::Continue(pos) => Err(EvalError::General(
            "Unexpected `continue` at top level".into(),
            pos,
        )),
    }
}