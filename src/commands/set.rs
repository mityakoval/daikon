use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;

use crate::data::redis_data::RedisData;
use crate::data::resp_types::RESPValue::{self, Err, SimpleString};
use crate::{
    DbMutex,
    data::{commands::CommandArray, resp_types::StoredValue},
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RESPValue> {
    let key = command_array.key.unwrap();

    let values = command_array.values.unwrap();

    let Some(RESPValue::BulkString(value)) = values.into_iter().next() else {
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
    if let Some(expiration) = command_array.ttl.map(|ttl| Instant::now() + ttl) {
        debug!("Setting ttl {:?}", &command_array.ttl);
        storage.expiry.insert(Arc::clone(&key), expiration);
    } else {
        storage.expiry.remove(&key);
    }
    Ok(SimpleString("OK".into()))
}
