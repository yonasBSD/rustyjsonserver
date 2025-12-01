use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::rjscript::evaluator::runtime::cache::GlobalCache;
use crate::rjscript::{
    evaluator::builtins::{
        arraycore::{array_methods_table, array_mut_methods_table},
        core::builtins_table,
        stringcore::string_methods_table,
        BuiltinFn, MutMethodFn, PureMethodFn,
    },
    semantics::{methods::receiver_from_vartype, methods::Receiver, types::VarType},
};
use crate::rjsdb::TableDb;

#[derive(Clone, Copy)]
pub enum MethodImpl {
    Pure(PureMethodFn),
    Mut(MutMethodFn),
}

#[derive(Clone)]
pub struct RuntimeGlobals {
    builtins: Arc<HashMap<String, BuiltinFn>>,
    methods: Arc<HashMap<(Receiver, String), MethodImpl>>,
    pub cache: Arc<GlobalCache>,
    pub db: Option<Arc<dyn TableDb>>,
}

static GLOBALS: OnceLock<Arc<RuntimeGlobals>> = OnceLock::new();

impl RuntimeGlobals {
    // single, process-wide instance
    fn build(db: Option<Arc<dyn TableDb>>) -> Arc<Self> {
        // Build builtins
        let builtins = builtins_table();

        // Unify all methods into one registry
        let mut methods: HashMap<(Receiver, String), MethodImpl> = HashMap::new();
        for (name, f) in string_methods_table().iter() {
            methods.insert((Receiver::String, name.clone()), MethodImpl::Pure(*f));
        }
        for (name, f) in array_methods_table().iter() {
            methods.insert((Receiver::Array, name.clone()), MethodImpl::Pure(*f));
        }
        for (name, f) in array_mut_methods_table().iter() {
            methods.insert((Receiver::Array, name.clone()), MethodImpl::Mut(*f));
        }

        Arc::new(RuntimeGlobals {
            builtins,
            methods: Arc::new(methods),
            cache: Arc::new(GlobalCache::new()),
            db,
        })
    }

    pub fn init_with_db(db: Option<Arc<dyn TableDb>>) -> Arc<Self> {
        GLOBALS.get_or_init(|| Self::build(db)).clone()
    }

    pub fn get() -> Arc<Self> {
        GLOBALS.get_or_init(|| Self::build(None)).clone()
    }

    #[inline]
    pub fn get_builtin(&self, name: &str) -> Option<&BuiltinFn> {
        self.builtins.get(name)
    }

    /// Resolve a method for a receiver type + name; choose mut vs pure impl.
    pub fn resolve_method(
        &self,
        recv_ty: &VarType,
        name: &str,
        wants_mut: bool,
    ) -> Option<MethodImpl> {
        let Some(rcv) = receiver_from_vartype(recv_ty) else {
            return None;
        };
        let key = (rcv, name.to_string());
        match (self.methods.get(&key), wants_mut) {
            (Some(MethodImpl::Mut(f)), true) => Some(MethodImpl::Mut(*f)),
            (Some(MethodImpl::Mut(_)), false) => None,
            (Some(MethodImpl::Pure(f)), _) => Some(MethodImpl::Pure(*f)),
            (None, _) => None,
        }
    }
}
