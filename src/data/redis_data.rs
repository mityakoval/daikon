use bytes::{Bytes, BytesMut};
use std::collections::VecDeque;

use crate::data::{
    errors::{StorageError, StorageResult},
    resp_types::RespValue,
};

pub(crate) const LP_MAX_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub(crate) struct ListPack {
    pub size: usize,
    pub data: BytesMut,
}

impl ListPack {
    pub(crate) fn push(&mut self, value: Bytes) -> StorageResult<()> {
        if self.size < LP_MAX_SIZE {
            let len_bytes = &(value.len() as u32).to_le_bytes();
            self.data.extend_from_slice(len_bytes);
            self.data.extend_from_slice(&value);
            self.data.extend_from_slice(len_bytes);
            self.size += 1;
            Ok(())
        } else {
            Err(StorageError::ListPackFull)
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &u8> {
        self.data.iter()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum RedisData {
    String(Bytes),
    List {
        packs: VecDeque<ListPack>,
        length: usize,
    },
}

impl RedisData {
    pub(crate) fn to_resp_value(&self) -> RespValue {
        match self {
            RedisData::String(bytes) => {
                RespValue::BulkString(String::from_utf8(bytes.to_vec()).unwrap())
            }
            _ => RespValue::Err("".into()),
        }
    }
}

pub(crate) struct RedisListIter<'a> {
    packs: &'a VecDeque<ListPack>,
    length: &'a usize,
    byte_offset: usize,
    item_offset: usize,
}

impl<'a> RedisListIter<'a> {
    pub(crate) fn new(packs: &'a VecDeque<ListPack>, length: &'a usize) -> Self {
        RedisListIter {
            packs,
            length,
            byte_offset: 0,
            item_offset: 0,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &u8> {
        self.packs.iter().flat_map(ListPack::iter)
    }
}

pub(crate) struct ListPosition {
    pack: usize,
    item: usize,
}

pub(crate) fn list_index_to_list_pack_index(i: usize, list: &VecDeque<ListPack>) -> ListPosition {
    ListPosition {
        pack: i / list.len(),
        item: i % list.len(),
    }
}
