use bytes::{Bytes, BytesMut};
use std::collections::VecDeque;

use crate::data::{errors::StorageError, resp_types::RespValue};

pub(crate) const LP_SEPARATOR: &str = "/r/n";
pub(crate) const LP_MAX_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub(crate) struct ListPack {
    pub size: usize,
    pub data: BytesMut,
}

impl ListPack {
    pub(crate) fn push(&mut self, value: String) -> Result<(), StorageError> {
        if self.size < LP_MAX_SIZE {
            self.data.extend_from_slice(&value.into_bytes());
            if LP_MAX_SIZE - self.size != 1 {
                self.data.extend_from_slice(LP_SEPARATOR.as_bytes());
            };
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

pub(crate) fn list_index_to_list_pack_index(
    i: i16,
    list: &VecDeque<ListPack>,
) -> Option<ListPosition> {
    list
}
