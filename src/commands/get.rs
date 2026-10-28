use std::sync::Arc;

use crate::{DbMutex, data::resp_types::RespValue};

pub(crate) fn invoke(storage_mutex: &DbMutex, key: Arc<str>) -> anyhow::Result<RespValue> {
    if let Ok(mut storage) = storage_mutex.lock() {
        if let Some(entry) = storage.get_live(&key) {
            return Ok(entry.value.to_resp_value());
        }
        debug!("No entry found");
        Ok(RespValue::NullBulkString())
    } else {
        Ok(RespValue::Err("Internal error occured".to_string()))
    }
}
