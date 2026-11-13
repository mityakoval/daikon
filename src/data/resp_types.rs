use std::collections::VecDeque;

use bytes::{BufMut, Bytes, BytesMut};

use crate::data::redis_data::RedisData;

pub trait RespType {
    fn encode(&mut self) -> BytesMut;
}

#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    Array(VecDeque<RespValue>),
    SimpleString(String),
    BulkString(Bytes),
    Integer(i64),
    NullBulkString(),
    Err(String),
}

#[derive(Debug)]
pub struct StoredValue {
    pub(crate) value: RedisData,
}

impl<'a> RespType for RespValue {
    fn encode(&mut self) -> BytesMut {
        let mut encoded: BytesMut = BytesMut::new();
        match self {
            RespValue::Array(array) => {
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .iter_mut()
                    .flat_map(|t| t.encode())
                    .for_each(|c| encoded.put_u8(c));
            }
            RespValue::SimpleString(value) => {
                encoded.extend_from_slice(format!("+{}\r\n", value).as_bytes());
            }
            RespValue::BulkString(value) => {
                encoded.extend_from_slice(format!("${}\r\n", value.len()).as_bytes());
                encoded.extend_from_slice(&value.to_vec());
                encoded.extend_from_slice("\r\n".as_bytes());
            }
            RespValue::Integer(value) => {
                encoded.extend_from_slice(format!(":{}\r\n", value).as_bytes());
            }
            RespValue::NullBulkString() => {
                encoded.extend_from_slice(b"$-1\r\n");
            }
            RespValue::Err(err_msg) => {
                encoded.extend_from_slice(format!("-ERR {}\r\n", err_msg).as_bytes());
            }
        };
        encoded
    }
}
