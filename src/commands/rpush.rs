use std::{cmp, collections::VecDeque, sync::Arc};

use bytes::BytesMut;

use crate::{
    DbMutex,
    data::{
        redis_data::{LP_MAX_SIZE, ListPack, RedisData},
        resp_types::{RespValue, StoredValue},
    },
};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    key: Arc<str>,
    values: &mut VecDeque<String>,
) -> anyhow::Result<RespValue> {
    let mut inserted: i64 = 0;
    if let Ok(mut storage) = storage_mut.lock() {
        if let Some(stored_value) = storage.get_mut_live(&key) {
            if let RedisData::List { packs, length: _ } = &mut stored_value.value {
                if let Some(last_pack) = packs.back_mut() {
                    let remaining_space = LP_MAX_SIZE - last_pack.size;
                    for string in values.drain(..remaining_space) {
                        last_pack.push(string);
                        inserted += 1;
                    }
                }
            }
        }

        /*
         * NEXT TODO:: When a list already exists, insert the pack into it and don't create a new
         * StoredValue
         */

        let len = values.len();
        let r = len % LP_MAX_SIZE;
        let mut packs = len / LP_MAX_SIZE;
        if r != 0 {
            packs += 1;
        }
        let list: VecDeque<ListPack> = VecDeque::with_capacity(packs);
        for _ in 0..packs {
            let until = cmp::min(len, LP_MAX_SIZE);
            let mut pack = ListPack {
                size: 0,
                data: BytesMut::new(),
            };
            for string in values.drain(..until) {
                pack.push(string);
            }
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

    Ok(RespValue::Integer(inserted as i64))
}
