use bytes::{BufMut, BytesMut};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use crate::data::errors::Error;
use crate::storage::Storage;



pub trait RESPType {
    fn encode(&mut self) -> BytesMut;
}

#[derive(Debug, Clone)]
pub enum Value {
    Array(Vec<Value>),
    SimpleString(String),
    BulkString(String),
    NullBulkString(),
}

impl RESPType for Value {
    fn encode(&mut self) -> BytesMut {
        let mut encoded: BytesMut = BytesMut::new();
        match self {
            Value::Array(array) => {
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .into_iter()
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
        };
        encoded
    }
}

