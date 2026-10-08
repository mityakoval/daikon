use std::{ops::Add, sync::Arc, time::Instant};

use crate::{
    DbMutex,
    data::{
        commands::CommandArray,
        redis_data::RedisData,
        resp_types::{
            RESPValue::{self, SimpleString},
            StoredValue,
        },
    },
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RESPValue> {
    if let Ok(mut storage) = storage_mutex.lock() {
        let key: Arc<str> = command_array.key.unwrap().into();
        let existing_expiration = storage.expiry.contains_key(&key);
        storage.data.insert(
            Arc::clone(&key),
            StoredValue {
                value: RedisData::String(command_array.value.unwrap()),
            },
        );
        if let Some(expiration) = command_array.ttl.map(|ttl| Instant::now().add(ttl)) {
            storage.expiry.insert(Arc::clone(&key), expiration);
        } else if existing_expiration {
            storage.expiry.remove(&key);
        }
        Ok(SimpleString("OK".into()))
    } else {
        Ok(RESPValue::Err("Internal error occured".to_string()))
    }
}
