use std::time::SystemTime;

use crate::{
    StorageMutex,
    data::{commands::CommandArray, types::Value},
};

pub(crate) fn invoke(
    storage_mutex: &StorageMutex,
    command_array: CommandArray,
) -> anyhow::Result<Value> {
    let key = command_array.key.unwrap();
    let now = SystemTime::now();
    let mut expired = false;
    let mut storage = storage_mutex.lock().unwrap();
    if let Some(entry) = storage.get(&key) {
        if entry.expires_at.is_none_or(|t| t > now) {
            return Ok(entry.value.clone());
        } else {
            expired = true;
        }
    }
    if expired {
        println!("Expired value for GET: {}. Removing", key);
        let key = storage.remove(&key).unwrap();
        println!("Removed {:?}", key)
    } else {
        println!("No entry found");
    }
    Ok(Value::NullBulkString())
}
