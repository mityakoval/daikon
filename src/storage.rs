use std::{collections::HashMap, time::Instant};

use bytes::Bytes;

use crate::data::resp_types::StoredValue;

pub struct Storage {
    pub data: HashMap<Bytes, StoredValue>,
    pub expiry: HashMap<Bytes, Instant>,
}

impl Storage {
    pub(crate) fn get_live(&mut self, key: &Bytes) -> Option<&StoredValue> {
        if !self.remove_if_expired(key) {
            self.data.get(key)
        } else {
            None
        }
    }

    pub(crate) fn get_mut_live(&mut self, key: &Bytes) -> Option<&mut StoredValue> {
        if !self.remove_if_expired(key) {
            self.data.get_mut(key)
        } else {
            None
        }
    }

    fn remove_if_expired(&mut self, key: &Bytes) -> bool {
        let expired = self
            .expiry
            .get(key)
            .is_some_and(|expires_at| *expires_at <= Instant::now());
        if expired {
            eprintln!("Found expired value for: {:?}. Removing", key);
            self.data.remove(key);
            self.expiry.remove(key);
            debug!("Removed {:?}", key);
        }
        expired
    }
}
