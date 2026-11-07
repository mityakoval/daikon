use bytes::{Bytes, BytesMut};
use std::collections::VecDeque;

use crate::data::{errors::StorageError, resp_types::RespValue};

pub(crate) const LP_MAX_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub(crate) struct ListPack {
    pub size: usize,
    pub data: BytesMut,
}

impl ListPack {
    pub(crate) fn push(&mut self, value: Bytes) -> Result<(), StorageError> {
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

pub(crate) struct ListPosition {
    pack: usize,
    item: usize,
}

//pub(crate) fn list_index_to_list_pack_index(
//    i: i16,
//    list: &VecDeque<ListPack>,
//) -> Option<ListPosition> {
//    list
//}
