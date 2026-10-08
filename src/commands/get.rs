use crate::{
    DbMutex,
    data::{commands::CommandArray, types::Value},
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<Value> {
    let key = command_array.key.unwrap();
    if let Ok(mut storage) = storage_mutex.lock() {
        if let Some(entry) = storage.get_live(&key) {
            return Ok(entry.value.clone());
        }
        println!("No entry found");
        Ok(Value::NullBulkString())
    } else {
        Ok(Value::Err("Internal error occured".to_string()))
    }
}
