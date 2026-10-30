use std::{collections::VecDeque, sync::Arc};

use crate::{
    DbMutex,
    data::{
        redis_data::{LP_MAX_SIZE, RedisData},
        resp_types::RespValue,
    },
};
use anyhow::anyhow;

pub(crate) fn invoke(
    storage_mutex: &DbMutex,
    key: Arc<str>,
    start: i16,
    stop: i16,
) -> anyhow::Result<RespValue> {
    if let Ok(mut storage) = storage_mutex.lock() {
        if let Some(stored_value) = storage.get_live(&key) {
            let RedisData::List(list_packs) = &stored_value.value else {
                return Err(anyhow!("Wrong type"));
            };

            let result = VecDeque::new();
            if list_packs.is_empty() {
                return Ok(RespValue::Array(result));
            };
            let list_len = (LP_MAX_SIZE * (list_packs.len() - 1))
                + list_packs.back().map_or(0, |pack| pack.size as usize);
            let from: usize = if start < 0 {
                list_packs.len() - (start.abs() as usize)
            } else {
                start as usize
            };

            let to: usize = if start < 0 {
                list_packs.len() - (stop.abs() as usize)
            } else {
                stop as usize
            };
        }
    }
}
