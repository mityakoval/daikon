use crate::data::commands::Command;
use crate::data::types::Value::NullBulkString;
use crate::data::types::Value::SimpleString;
use crate::data::types::{RESPType, StoredValue, Value};
use crate::parser::commands::parse_command;
use bytes::BytesMut;
use dashmap::DashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(crate) mod data;
pub(crate) mod parser;
pub mod storage;

pub async fn handle_connection(mut stream: TcpStream, storage: Arc<DashMap<String, StoredValue>>) {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        match stream.read_buf(&mut buf).await {
            Ok(size) => {
                if size == 0 {
                    break;
                }

                match parse_command(&mut buf) {
                    Ok(command) => {
                        let result = execute_command(command, &storage).unwrap();
                        respond(&mut stream, result).await;
                    }
                    Err(e) => {
                        eprintln!("Error parsing command: {:#}", e);
                        stream
                            .write_all("-Unknown command\r\n".as_bytes())
                            .await
                            .expect("Could not send error");
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                break;
            }
        }
    }
}

fn execute_command(
    command: Command,
    storage: &Arc<DashMap<String, StoredValue>>,
) -> anyhow::Result<Value> {
    match command {
        Command::ECHO(value) => Ok(value),
        Command::PING => Ok(SimpleString("PONG".into())),
        Command::SET { key, value, ttl } => {
            storage.insert(
                key,
                StoredValue {
                    value,
                    expires_at: match ttl {
                        None => None,
                        Some(ttl) => Some(SystemTime::now().add(ttl)),
                    },
                },
            );
            Ok(SimpleString("OK".into()))
        }
        Command::GET(key) => {
            let now = SystemTime::now();
            if let Some(entry)  = storage.get(&key) {
                    if entry.expires_at.map_or(true, |t| t > now) {
                        return Ok(entry.value.clone())
                    } else {
                        eprintln!("Entry expired");
                        let (key, value) = storage.remove(&key).unwrap();
                        eprintln!("Removed {}", key);
                    }
            }
            eprintln!("No entry found");
            Ok(NullBulkString())
        }
    }
}

async fn respond<V>(stream: &mut TcpStream, mut value: V)
where
    V: RESPType,
{
    match stream.write_buf(&mut value.encode()).await {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error writing to stream: {}", e)
        }
    }
}
