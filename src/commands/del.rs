use std::{ops::Add, sync::Arc};

use crate::{DbMutex, data::resp_types::RespValue};

pub(crate) fn invoke(storage_mutex: &DbMutex, keys: Vec<Arc<str>>) -> anyhow::Result<RespValue> {
    if let Ok(mut storage) = storage_mutex.lock() {
        let removed = keys.iter().fold(0i64, |acc, k| {
            if storage.get_live(k).is_some() {
                storage.data.remove(k);
                storage.expiry.remove(k);
                debug!("Removed {:?}", k);
                acc.add(1)
            } else {
                acc.add(0)
            }
        });
        Ok(RespValue::Integer(removed))
    } else {
        Ok(RespValue::Err("Internal error occured".to_string()))
    }
}
