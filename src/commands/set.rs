use std::ops::Add;
use std::sync::Arc;
use std::time::Instant;

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
    if let Ok(mut storage) = storage_mutex.lock() {
        let key: Arc<str> = command_array.key.unwrap().into();
        let existing_expiration = storage.expiry.contains_key(&key);
        if let Ok(redis_data) = RedisData::from_resp_value(command_array.value.unwrap()) {
            storage
                .data
                .insert(Arc::clone(&key), StoredValue { value: redis_data });
            if let Some(expiration) = command_array.ttl.map(|ttl| Instant::now().add(ttl)) {
                storage.expiry.insert(Arc::clone(&key), expiration);
            } else if existing_expiration {
                storage.expiry.remove(&key);
            }
            Ok(SimpleString("OK".into()))
        } else {
            Ok(Err("Value cannot be stored".into()))
        }
    } else {
        Ok(RESPValue::Err("Internal error occured".to_string()))
    }
}
