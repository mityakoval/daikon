use bytes::Bytes;
use std::collections::VecDeque;

use crate::data::resp_types::RESPValue;

pub(crate) const LP_SEPARATOR: &'static str = ":";
pub(crate) const LP_MAX_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub struct ListPack {
    pub size: u8,
    pub data: Bytes,
}

#[derive(Debug, Clone)]
pub enum RedisData {
    String(Bytes),
    List(VecDeque<ListPack>),
}

impl RedisData {
    pub(crate) fn to_resp_value(&self) -> RESPValue {
        match self {
            RedisData::String(bytes) => {
                RESPValue::BulkString(String::from_utf8(bytes.to_vec()).unwrap())
            }
            _ => RESPValue::Err("".into()),
        }
    }
}
