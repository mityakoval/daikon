use crate::data::commands::{Command, RedisCommand};
use crate::data::types::{RESPType, Value};
use crate::parser::parse_command;
use crate::storage::Storage;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(crate) mod data;
pub(crate) mod parser;
pub mod storage;

pub async fn handle_connection(mut stream: TcpStream, storage: Arc<Storage>) {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        match stream.read_buf(&mut buf).await {
            Ok(size) => {
                if size == 0 {
                    break;
                }

                match parse_command(&mut buf) {
                    Ok(command) => {
                        let result = execute_command(command, &storage).await.unwrap();
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

async fn execute_command(command: Command, storage: &Arc<Storage>) -> anyhow::Result<Value> {
    match command {
        Command::ECHO(value) => Ok(value),
        Command::PING => { Ok(Value::SimpleString("PONG".into())) }
        Command::SET(String, Value) => {
            storage
        }
    }
}

async fn respond<V>(stream: &mut TcpStream, mut value: V) where V: RESPType {
    match stream.write_buf(&mut value.encode()).await {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error writing to stream: {}", e)
        }
    }
}
