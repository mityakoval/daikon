use bytes::{BufMut, BytesMut};
use std::time::SystemTime;

pub trait RESPType {
    fn encode(&mut self) -> BytesMut;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Array(Vec<Value>),
    SimpleString(String),
    BulkString(String),
    NullBulkString(),
    Err(String),
}

pub struct StoredValue {
    pub value: Value,
    pub expires_at: Option<SystemTime>,
}

impl StoredValue {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= SystemTime::now())
    }
}

impl RESPType for Value {
    fn encode(&mut self) -> BytesMut {
        let mut encoded: BytesMut = BytesMut::new();
        match self {
            Value::Array(array) => {
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .iter_mut()
                    .flat_map(|t| t.encode())
                    .for_each(|c| encoded.put_u8(c));
            }
            Value::SimpleString(value) => {
                encoded.extend_from_slice(format!("+{}\r\n", value).as_bytes());
            }
            Value::BulkString(value) => {
                encoded.extend_from_slice(format!("${}\r\n{}\r\n", value.len(), value).as_bytes());
            }
            Value::NullBulkString() => {
                encoded.extend_from_slice(b"$-1\r\n");
            }
            Value::Err(err_msg) => {
                encoded.extend_from_slice(format!("-{}\r\n", err_msg).as_bytes());
            }
        };
        encoded
    }
}

