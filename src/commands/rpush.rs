use std::{cmp, collections::VecDeque, sync::Arc};

use crate::{
    DbMutex,
    data::{
        redis_data::{LP_MAX_SIZE, ListPack, RedisData},
        resp_types::{RespValue, StoredValue},
    },
};
use anyhow::anyhow;
use bytes::{Bytes, BytesMut};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    key: Arc<str>,
    values: &mut VecDeque<Bytes>,
) -> anyhow::Result<RespValue> {
    if let Ok(mut storage) = storage_mut.lock() {
        if let Some(stored_value) = storage.get_mut_live(&key) {
            if let RedisData::List { packs, length } = &mut stored_value.value {
                if let Some(last_pack) = packs.back_mut() {
                    let remaining_space = LP_MAX_SIZE - last_pack.size;
                    let until = cmp::min(values.len(), remaining_space);
                    for string in values.drain(..until) {
                        last_pack.push(string);
                        *length += 1;
                    }
                }
                let inserted = pack(packs, values);
                *length += inserted;
                return Ok(RespValue::Integer(*length as i64));
            } else {
                return Err(anyhow!("WRONTYPE"));
            }
        } else {
            let mut list = VecDeque::new();
            let inserted = pack(&mut list, values);
            storage.data.insert(
                key,
                StoredValue {
                    value: RedisData::List {
                        length: inserted,
                        packs: list,
                    },
                },
            );
            return Ok(RespValue::Integer(inserted as i64));
        }
    }
    Err(anyhow!("Inaccessible storage"))
}

fn pack(
    list: &mut VecDeque<ListPack>,
    values: &mut VecDeque<Bytes>,
) -> Result<usize, StorageError> {
    let mut inserted: usize = 0;
    let len = values.len();
    let new_packs = len.div_ceil(LP_MAX_SIZE);
    list.reserve(new_packs);
    for _ in 0..new_packs {
        let until = cmp::min(values.len(), LP_MAX_SIZE);
        let mut pack = ListPack {
            size: 0,
            data: BytesMut::new(),
        };
        for string in values.drain(..until) {
            pack.push(string);
            inserted += 1;
        }
        list.push_back(pack);
    }
    inserted
}

#[cfg(test)]

mod test {
    use crate::commands::rpush::pack;

    #[test]
    fn pack_works_when_there_is_space_in_listpack() {
        pack(list, values)
    }
}
