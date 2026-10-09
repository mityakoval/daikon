use anyhow::anyhow;
use bytes::Bytes;
use std::collections::VecDeque;

use crate::data::resp_types::RESPValue;

#[derive(Debug, Clone)]
pub enum RedisData {
    String(Bytes),
    List(VecDeque<Bytes>),
}

impl RedisData {
    pub(crate) fn from_resp_value(resp_value: RESPValue) -> anyhow::Result<Self> {
        match resp_value {
            RESPValue::BulkString(value) => Ok(RedisData::String(Bytes::from(value))),
            _ => Err(anyhow!("This value cannot be stored")),
        }
    }

    pub(crate) fn to_resp_value(&self) -> RESPValue {
        match self {
            RedisData::String(bytes) => {
                RESPValue::BulkString(String::from_utf8(bytes.to_vec()).unwrap())
            }
            _ => RESPValue::Err("".into()),
        }
    }
}
