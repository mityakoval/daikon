use crate::data::commands::{Command, CommandArray};
use crate::data::types::{self, RESPType, StoredValue, Value};
use crate::parser::commands::parse_command_array;
use bytes::BytesMut;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(crate) mod commands;
pub(crate) mod data;
pub(crate) mod parser;

type StorageMutex = Arc<Mutex<HashMap<String, StoredValue>>>;

pub async fn handle_connection(mut stream: TcpStream, storage: StorageMutex) {
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

fn execute_command(command_array: CommandArray, storage: &StorageMutex) -> anyhow::Result<Value> {
    match command_array.command {
        Command::Ping => Ok(command_array.value.unwrap()),
        Command::Echo => Ok(command_array.value.unwrap()),
        Command::Set => commands::set::invoke(storage, command_array),
        Command::Get => commands::get::invoke(storage, command_array),
        //Command::RPUSH => {}
        _ => Ok(types::Value::Err("Unknown command".into())),
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
