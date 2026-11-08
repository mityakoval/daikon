use std::{cmp, collections::VecDeque, sync::Arc};

use crate::{
    DbMutex,
    data::{
        errors::{StorageError, StorageResult},
        redis_data::{LP_MAX_SIZE, ListPack, RedisData},
        resp_types::{RespValue, StoredValue},
    },
};
use bytes::{Bytes, BytesMut};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    key: Arc<str>,
    values: &mut VecDeque<Bytes>,
) -> StorageResult<RespValue> {
    if let Ok(mut storage) = storage_mut.lock() {
        if let Some(stored_value) = storage.get_mut_live(&key) {
            if let RedisData::List { packs, length } = &mut stored_value.value {
                if let Some(last_pack) = packs.back_mut() {
                    let remaining_space = LP_MAX_SIZE - last_pack.size;
                    let until = cmp::min(values.len(), remaining_space);
                    for string in values.drain(..until) {
                        last_pack.push(string)?;
                        *length += 1;
                    }
                }
                let inserted = pack(packs, values)?;
                *length += inserted;
                return Ok(RespValue::Integer(*length as i64));
            } else {
                return Err(StorageError::WrongType);
            }
        } else {
            let mut list = VecDeque::new();
            let inserted = pack(&mut list, values)?;
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
    Err(StorageError::InaccessibleStorage)
}

fn pack(list: &mut VecDeque<ListPack>, values: &mut VecDeque<Bytes>) -> StorageResult<usize> {
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
            pack.push(string)?;
            inserted += 1;
        }
        list.push_back(pack);
    }
    Ok(inserted)
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, VecDeque},
        panic,
        sync::{Arc, Mutex},
    };

    use bytes::Bytes;

    use crate::{
        commands::rpush::{invoke, pack},
        data::{
            redis_data::{ListPack, RedisData},
            resp_types::{RespValue, StoredValue},
        },
        storage::Storage,
    };

    #[test]
    fn pack_works() {
        let mut list: VecDeque<ListPack> = VecDeque::new();
        let mut values = VecDeque::from([
            Bytes::from("one"),
            Bytes::from("two"),
            Bytes::from("three"),
            Bytes::from("four"),
        ]);
        pack(&mut list, &mut values).expect("expected OK");

        let listpack = list.back().expect("expected a ListPack");
        assert_eq!(listpack.size, 4);
        assert_eq!(
            listpack.data.to_vec(),
            b"\x03\x00\x00\x00one\x03\x00\x00\x00\x03\x00\x00\x00two\x03\x00\x00\x00\x05\x00\x00\x00three\x05\x00\x00\x00\x04\x00\x00\x00four\x04\x00\x00\x00"
        )
    }

    #[test]
    fn pack_adds_multiple_list_packs() {
        let mut list: VecDeque<ListPack> = VecDeque::new();
        let size = 33;
        let mut values = VecDeque::with_capacity(size);
        for n in 0..size {
            values.push_back(Bytes::from(n.to_string()));
        }
        let inserted = pack(&mut list, &mut values).expect("expected StorageError");
        assert_eq!(list.len(), 4);
        assert_eq!(inserted, size);
        let last_listpack = list.back().expect("expected a ListPack");
        assert_eq!(last_listpack.size, 3);
    }

    #[test]
    fn rpush_adds_items_to_the_list() {
        let storage_mutex = Arc::new(Mutex::new(Storage {
            data: HashMap::new(),
            expiry: HashMap::new(),
        }));
        let key = Arc::<str>::from("key");

        let size = 33;
        let mut values = VecDeque::with_capacity(size);
        for n in 0..size {
            values.push_back(Bytes::from(n.to_string()));
        }

        let inserted = invoke(&storage_mutex, Arc::clone(&key), &mut values)
            .expect("expected number of inserted items");
        let RespValue::Integer(n) = inserted else {
            panic!("expected RespValue::Integer");
        };

        assert_eq!(n as usize, size);
        let add_size = 7;

        for n in 0..add_size {
            values.push_back(Bytes::from(n.to_string()));
        }

        let inserted = invoke(&storage_mutex, Arc::clone(&key), &mut values)
            .expect("expected number of inserted items");
        let RespValue::Integer(n) = inserted else {
            panic!("expected RespValue::Integer");
        };
        assert_eq!(n as usize, size + add_size);

        let Ok(mut storage) = storage_mutex.lock() else {
            panic!("couldn't obtain storage lock");
        };

        let Some(StoredValue {
            value: RedisData::List { packs, length },
        }) = storage.get_live(&key)
        else {
            panic!("expected stored value");
        };
        assert_eq!(*length, size + add_size);
        assert_eq!(packs.len(), 4);
    }
}
