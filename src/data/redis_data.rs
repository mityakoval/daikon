use bytes::Bytes;
use std::collections::VecDeque;

use crate::data::resp_types::RespValue;

pub(crate) const LP_SEPARATOR: &str = "/r/n";
pub(crate) const LP_MAX_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub(crate) struct ListPack {
    pub size: usize,
    pub data: Bytes,
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
