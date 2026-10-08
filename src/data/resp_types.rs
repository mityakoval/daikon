use bytes::{BufMut, BytesMut};

use crate::data::redis_data::RedisData;

pub trait RESPType {
    fn encode(&mut self) -> BytesMut;
}

#[derive(Debug, Clone, PartialEq)]
pub enum RESPValue {
    Array(Vec<RESPValue>),
    SimpleString(String),
    BulkString(String),
    NullBulkString(),
    Err(String),
}

#[derive(Debug)]
pub struct StoredValue {
    pub value: RedisData,
}

impl RESPType for RESPValue {
    fn encode(&mut self) -> BytesMut {
        let mut encoded: BytesMut = BytesMut::new();
        match self {
            RESPValue::Array(array) => {
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .iter_mut()
                    .flat_map(|t| t.encode())
                    .for_each(|c| encoded.put_u8(c));
            }
            RESPValue::SimpleString(value) => {
                encoded.extend_from_slice(format!("+{}\r\n", value).as_bytes());
            }
            RESPValue::BulkString(value) => {
                encoded.extend_from_slice(format!("${}\r\n{}\r\n", value.len(), value).as_bytes());
            }
            RESPValue::NullBulkString() => {
                encoded.extend_from_slice(b"$-1\r\n");
            }
            RESPValue::Err(err_msg) => {
                encoded.extend_from_slice(format!("-{}\r\n", err_msg).as_bytes());
            }
        };
        encoded
    }
}
