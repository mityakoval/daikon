use crate::data::commands::{CommandArray, CommandName};
use crate::data::resp_types::{self, RESPType, RESPValue};
use crate::parser::commands::parse_command_array;
use crate::storage::Storage;
use bytes::BytesMut;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub static DEBUG: AtomicBool = AtomicBool::new(false);

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::DEBUG.load(::std::sync::atomic::Ordering::Relaxed) { eprintln!($($arg)*); }
    };
}

pub(crate) mod commands;
pub(crate) mod data;
pub(crate) mod parser;
pub mod storage;

type DbMutex = Arc<Mutex<Storage>>;

pub async fn handle_connection(mut stream: TcpStream, db_mutex: DbMutex) {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        match stream.read_buf(&mut buf).await {
            Ok(size) => {
                if size == 0 {
                    break;
                }
                debug!("========================");
                debug!("= RECEIVED NEW REQUEST =");
                debug!("========================");
                debug!("\nREQUEST: {:?}\n", buf);

                match parse_command_array(&mut buf) {
                    Ok(command) => {
                        eprintln!("Received command: {:?}\n🔄 Executing...", command);
                        let result = execute_command(command, &db_mutex).unwrap();
                        eprintln!("✅ Done!");
                        respond(&mut stream, result).await;
                    }
                    Err(e) => {
                        eprintln!("⚠️ Error parsing command: {:#}", e);
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

fn execute_command(command_array: CommandArray, storage: &DbMutex) -> anyhow::Result<RESPValue> {
    match command_array.command.name {
        CommandName::Ping => Ok(commands::ping::invoke()),
        CommandName::Echo => commands::echo::invoke(command_array),
        CommandName::Set => commands::set::invoke(storage, command_array),
        CommandName::Get => commands::get::invoke(storage, command_array),
        CommandName::RPush => commands::rpush::invoke(storage, command_array),
        _ => Ok(resp_types::RESPValue::Err("Unknown command".into())),
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
