use std::{cmp, collections::VecDeque};

use anyhow::anyhow;

use bytes::BytesMut;

use crate::{
    DbMutex,
    data::{
        commands::CommandArray,
        redis_data::{LP_MAX_SIZE, LP_SEPARATOR, ListPack, RedisData},
        resp_types::{RESPValue, StoredValue},
    },
};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RESPValue> {
    let mut values = command_array.values.unwrap();
    let r = values.len() % LP_MAX_SIZE;
    let mut packs = values.len() / LP_MAX_SIZE;
    if r != 0 {
        packs += 1;
    }
    let mut list: VecDeque<ListPack> = VecDeque::with_capacity(packs);
    for _ in 0..packs {
        let until = cmp::min(values.len(), LP_MAX_SIZE);
        let mut pack = BytesMut::new();
        for v in values.drain(..until) {
            let RESPValue::BulkString(string) = v else {
                debug!("RPUSH: Not a bulk string");
                return Err(anyhow!("wrong argument provided"));
            };
            pack.extend_from_slice(&string.into_bytes());
            pack.extend_from_slice(LP_SEPARATOR.as_bytes());
        }
        list.push_back(ListPack {
            size: until as u8,
            data: pack.freeze(),
        });
    }
    let key = command_array.keys.unwrap().pop().unwrap();
    if let Ok(mut storage) = storage_mut.lock() {
        storage.data.insert(
            key,
            StoredValue {
                value: RedisData::List(list),
            },
        );
    }

    Ok(RESPValue::Integer(values.len() as i64))
}
