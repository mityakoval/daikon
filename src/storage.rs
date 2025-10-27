use dashmap::DashMap;
use crate::data::types::Value;

pub struct Storage {
    storage: DashMap<String, Value>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
        }
    }
}