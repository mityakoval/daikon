use std::{cmp, collections::VecDeque, sync::Arc};

use anyhow::anyhow;

use bytes::BytesMut;

use crate::{
    DbMutex,
    data::{
        redis_data::{LP_MAX_SIZE, LP_SEPARATOR, ListPack, RedisData},
        resp_types::{RespValue, StoredValue},
    },
};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    key: Arc<str>,
    values: &mut VecDeque<RespValue>,
) -> anyhow::Result<RespValue> {
    let len = values.len();
    if let Ok(mut storage) = storage_mut.lock() {
        let mut list: VecDeque<ListPack>;
        if let Some(stored_value) = storage.get_mut_live(&key) {
            if let RedisData::List { packs, length } = &mut stored_value.value {}
        } else {
            let r = len % LP_MAX_SIZE;
            let mut packs = len / LP_MAX_SIZE;
            if r != 0 {
                packs += 1;
            }
            let mut list: VecDeque<ListPack> = VecDeque::with_capacity(packs);
            for _ in 0..packs {
                let until = cmp::min(len, LP_MAX_SIZE);
                let mut pack = BytesMut::new();
                for v in values.drain(..until) {
                    let RespValue::BulkString(string) = v else {
                        debug!("RPUSH: Not a bulk string");
                        return Err(anyhow!("wrong argument provided"));
                    };
                    pack.extend_from_slice(&string.into_bytes());
                    pack.extend_from_slice(LP_SEPARATOR.as_bytes());
                }
                list.push_back(ListPack {
                    size: until,
                    data: pack.freeze(),
                });
            }
            storage.data.insert(
                key,
                StoredValue {
                    value: RedisData::List {
                        length: len,
                        packs: list,
                    },
                },
            );
        }
    }

    Ok(RespValue::Integer(len as i64))
}
