use std::{collections::VecDeque, ops::Div, sync::Arc};

use crate::{
    DbMutex, data::{
        errors::{StorageError, StorageResult}, redis_data::{LP_MAX_SIZE, RedisData, RedisListIter, list_index_to_list_pack_index}, resp_types::RespValue,
    },
};

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    key: Arc<str>,
    start: i16,
    stop: i16,
) -> StorageResult<RespValue> {
    if let Ok(mut storage) = storage_mutex.lock() {
        if let Some(stored_value) = storage.get_live(&key) {
            let RedisData::List { packs, length } = &stored_value.value else {
                return Err(StorageError::WrongType);
            };

            let result = VecDeque::new();
            if packs.is_empty() {
                return Ok(RespValue::Array(result));
            };
            let from: usize = if start < 0 {
                *length - (start.unsigned_abs() as usize)
            } else {
                start as usize
            };

            let from = list_index_to_list_pack_index(from, packs);

            let to: usize = if start < 0 {
                *length - (stop.unsigned_abs() as usize)
            } else {
                stop as usize
            };
            let to = list_index_to_list_pack_index(to, packs);

            let res = RedisListIter::new()
        };
    } else {
        return Err(StorageError::InaccessibleStorage);
    }
    Ok(RespValue::NullBulkString())
}
