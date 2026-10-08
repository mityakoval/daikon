use crate::{
    DbMutex,
    data::{commands::CommandArray, resp_types::RESPValue},
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RESPValue> {
    let key = command_array.key.unwrap();
    if let Ok(mut storage) = storage_mutex.lock() {
        if let Some(entry) = storage.get_live(&key) {
            return Ok(entry.value.clone());
        }
        println!("No entry found");
        Ok(RESPValue::NullBulkString())
    } else {
        Ok(RESPValue::Err("Internal error occured".to_string()))
    }
}
