use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::data::resp_types::StoredValue;

pub struct Storage {
    pub data: HashMap<Arc<str>, StoredValue>,
    pub expiry: HashMap<Arc<str>, Instant>,
}

impl Storage {
    pub(crate) fn get_live(&mut self, key: &str) -> Option<&StoredValue> {
        if !self.remove_if_expired(key) {
            self.data.get(key)
        } else {
            None
        }
    }

    pub(crate) fn get_mut_live(&mut self, key: &str) -> Option<&mut StoredValue> {
        if !self.remove_if_expired(key) {
            self.data.get_mut(key)
        } else {
            None
        }
    }

    fn remove_if_expired(&mut self, key: &str) -> bool {
        let expired = self
            .expiry
            .get(key)
            .is_some_and(|expires_at| *expires_at <= Instant::now());
        if expired {
            eprintln!("Found expired value for: {}. Removing", key);
            self.data.remove(key);
            self.expiry.remove(key);
            debug!("Removed {:?}", key);
        }
        expired
    }
}
