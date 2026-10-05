use crate::data::commands::{Command, CommandArray};
use crate::data::types::Value::NullBulkString;
use crate::data::types::Value::SimpleString;
use crate::data::types::{self, RESPType, StoredValue, Value};
use crate::parser::commands::parse_command_array;
use bytes::BytesMut;
use dashmap::DashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(crate) mod commands;
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

                match parse_command_array(&mut buf) {
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
    command_array: CommandArray,
    storage: &Arc<DashMap<String, StoredValue>>,
) -> anyhow::Result<Value> {
    match command_array.command {
        Command::PING => Ok(command_array.value.unwrap()),
        Command::ECHO => Ok(command_array.value.unwrap()),
        Command::SET => {
            storage.insert(
                command_array.key.unwrap(),
                StoredValue {
                    value: command_array.value.unwrap(),
                    expires_at: command_array.ttl.map(|ttl| SystemTime::now().add(ttl)),
                },
            );
            Ok(SimpleString("OK".into()))
        }
        Command::GET => {
            // let stored_value = storage.get(&command_array.key.unwrap()).unwrap();
            // if stored_value.is_expired() {
            //
            // } else {
            //     Ok(stored_value.value)
            // }

            let key = command_array.key.unwrap();
            let now = SystemTime::now();
            let mut expired = false;
            if let Some(entry) = storage.get(&key) {
                if entry.expires_at.is_none_or(|t| t > now) {
                    return Ok(entry.value.clone());
                } else {
                    expired = true;
                }
            }
            if expired {
                eprintln!("Expired value for GET: {}. Removing", key);
                let (key, _) = storage.remove(&key).unwrap();
                eprintln!("Removed {}", key)
            } else {
                eprintln!("No entry found");
            }
            Ok(NullBulkString())
        }
        //Command::RPUSH => {}
        _ => Ok(types::Value::Err("Unknown command".into())),
    }

    // match command_array.command {
    //     Command::ECHO => Ok(command_array.value.unwrap()),
    //     Command::PING => Ok(SimpleString("PONG".into())),
    //     Command::SET { key, value, ttl } => {
    //         storage.insert(
    //             key,
    //             StoredValue {
    //                 value,
    //                 expires_at: ttl.map(|dur| SystemTime::now().add(dur)),
    //             },
    //         );
    //         Ok(SimpleString("OK".into()))
    //     }
    //     Command::GET(key) => {
    //         let now = SystemTime::now();
    //         let mut expired = false;
    //         if let Some(entry)  = storage.get(&key) {
    //                 if entry.expires_at.map_or(true, |t| t > now) {
    //                     return Ok(entry.value.clone())
    //                 } else {
    //                     expired = true;
    //                 }
    //         }
    //         if expired {
    //             eprintln!("Expired value for GET: {}. Removing", key);
    //             let (key, value) = storage.remove(&key).unwrap();
    //             eprintln!("Removed {}", key)
    //         } else {
    //             eprintln!("No entry found");
    //         }
    //         Ok(NullBulkString())
    //     }
    // }
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
