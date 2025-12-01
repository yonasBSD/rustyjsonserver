use std::sync::Arc;

use crate::rjscript::evaluator::runtime::{request_cache::RequestCache, runtime_globals::RuntimeGlobals};


#[derive(Clone)]
pub struct EvalCtx {
    pub globals: Arc<RuntimeGlobals>,
    pub req: Arc<RequestCache>,
}

impl EvalCtx {
    pub fn new(globals: Arc<RuntimeGlobals>, req: Arc<RequestCache>) -> Self {
        Self { globals, req }
    }
}
