use std::ops::Add;

use crate::{
    DbMutex,
    data::{commands::CommandArray, resp_types::RespValue},
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RespValue> {
    if let Ok(mut storage) = storage_mutex.lock() {
        let removed = command_array.keys.unwrap().iter().fold(0i64, |acc, k| {
            if storage.get_live(k).is_some() {
                storage.data.remove(k);
                debug!("Removed {:?}", k);
                acc.add(1)
            } else {
                acc.add(0)
            }
        });
        Ok(RespValue::Integer(removed))
    } else {
        Ok(RespValue::Err("Internal error occured".to_string()))
    }
}
