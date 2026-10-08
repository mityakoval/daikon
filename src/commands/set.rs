use std::{ops::Add, sync::Arc, time::Instant};

use crate::{
    DbMutex,
    data::{
        commands::CommandArray,
        types::{
            StoredValue,
            Value::{self, SimpleString},
        },
    },
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<Value> {
    if let Ok(mut storage) = storage_mutex.lock() {
        let key: Arc<str> = command_array.key.unwrap().into();
        storage.data.insert(
            Arc::clone(&key),
            StoredValue {
                value: command_array.value.unwrap(),
            },
        );
        if let Some(expiration) = command_array.ttl.map(|ttl| Instant::now().add(ttl)) {
            storage.expiry.insert(Arc::clone(&key), expiration);
        }
        Ok(SimpleString("OK".into()))
    } else {
        Ok(Value::Err("Internal error occured".to_string()))
    }
}
