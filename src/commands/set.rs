use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;

use crate::data::redis_data::RedisData;
use crate::data::resp_types::RespValue::{self, Err, SimpleString};
use crate::{DbMutex, data::resp_types::StoredValue};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    key: Arc<str>,
    value: RespValue,
    ttl: Option<Duration>,
) -> anyhow::Result<RespValue> {
    let RespValue::BulkString(value) = value else {
        return Ok(Err("Value cannot be stored".into()));
    };

    let redis_data = RedisData::String(Bytes::from(value));

    let Ok(mut storage) = storage_mutex.lock() else {
        return Ok(Err("Internal error occurred".to_string()));
    };
    debug!("Saving value '{:?}' under key '{}'.", redis_data, key);
    storage
        .data
        .insert(Arc::clone(&key), StoredValue { value: redis_data });
    if let Some(expiration) = ttl.map(|ttl| Instant::now() + ttl) {
        debug!("Setting ttl {:?}", ttl);
        storage.expiry.insert(Arc::clone(&key), expiration);
    } else {
        storage.expiry.remove(&key);
    }
    Ok(SimpleString("OK".into()))
}
