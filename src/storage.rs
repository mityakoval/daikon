use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::data::redis_data::RedisData;

pub struct Storage {
    pub data: HashMap<Arc<str>, RedisData>,
    pub expiry: HashMap<Arc<str>, Instant>,
}

impl Storage {
    pub(crate) fn get_live(&mut self, key: &str) -> Option<&RedisData> {
        let expired = self
            .expiry
            .get(key)
            .is_some_and(|expires_at| *expires_at <= Instant::now());
        if expired {
            println!("Found expired value for: {}. Removing", key);
            self.data.remove(key);
            self.expiry.remove(key);
            println!("Removed {:?}", key);
            None
        } else {
            self.data.get(key)
        }
    }
}
