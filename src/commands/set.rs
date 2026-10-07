use std::{ops::Add, time::Instant};

use crate::{
    StorageMutex,
    data::{
        commands::CommandArray,
        types::{
            StoredValue,
            Value::{self, SimpleString},
        },
    },
};

pub(crate) fn invoke(
    storage_mutex: &StorageMutex,
    command_array: CommandArray,
) -> anyhow::Result<Value> {
    let mut storage = storage_mutex.lock().unwrap();
    storage.insert(
        command_array.key.unwrap(),
        StoredValue {
            value: command_array.value.unwrap(),
            expires_at: command_array.ttl.map(|ttl| Instant::now().add(ttl)),
        },
    );
    Ok(SimpleString("OK".into()))
}
