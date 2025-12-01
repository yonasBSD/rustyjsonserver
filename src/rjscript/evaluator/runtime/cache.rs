use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::rjscript::evaluator::runtime::value::RJSValue;

#[derive(Clone)]
pub struct GlobalCache {
    map: Arc<RwLock<HashMap<String, RJSValue>>>,
}

impl GlobalCache {
    pub fn new() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<RJSValue> {
        self.map.read().ok().and_then(|guard| guard.get(key).cloned())
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.read().map(|guard| guard.contains_key(key)).unwrap_or(false)
    }

    pub fn set(&self, key: String, value: RJSValue) {
        if let Ok(mut guard) = self.map.write() {
            guard.insert(key, value);
        }
    }

    pub fn del(&self, key: &str) -> bool {
        self.map.write().ok().and_then(|mut guard| guard.remove(key)).is_some()
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.map.write() {
            guard.clear();
        }
    }
}
