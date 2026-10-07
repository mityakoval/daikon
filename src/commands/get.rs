use std::time::SystemTime;

use crate::{
    data::{commands::CommandArray, types::Value},
    Storage,
};

pub(crate) fn invoke(storage: &Storage, command_array: CommandArray) -> anyhow::Result<Value> {
    let key = command_array.key.unwrap();
    let now = SystemTime::now();
    let mut expired = false;
    if let Some(entry) = storage.get(&key) {
        if entry.expires_at.is_none_or(|t| t > now) {
            return Ok(entry.value.clone());
        } else {
            expired = true;
        }
    }
    if expired {
        eprintln!("Expired value for GET: {}. Removing", key);
        let (key, _) = storage.remove(&key).unwrap();
        eprintln!("Removed {}", key)
    } else {
        eprintln!("No entry found");
    }
    Ok(Value::NullBulkString())
}
