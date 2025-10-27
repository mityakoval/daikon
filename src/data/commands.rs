use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use crate::data::errors::Error;
use crate::data::types::{RESPType, Value};
use crate::storage::Storage;

pub enum Command {
    PING,
    ECHO(Value),
    SET(String, Value),
    GET(String),
}

pub trait RedisCommand {
    fn execute(&mut self, storage: &mut Storage) -> Result<Option<Value>, Error>;
}

pub struct PING;
pub struct ECHO {
    pub(crate) value: Value
}
pub struct SET {
    pub(crate) value: Value
}
pub struct GET {
    key: String
}
impl RedisCommand for PING {
    fn execute(&mut self, _: &mut Storage) -> Result<Option<Value>, Error> {
        Ok(Some(Value::SimpleString("PONG".into())))
    }
}

impl RedisCommand for ECHO {
    fn execute(&mut self, _: &mut Storage) -> Result<Option<Value>, Error> {
        Ok(Some(self.value.clone()))
    }
}

impl RedisCommand for SET {
    fn execute(&mut self, storage: &mut Storage) -> Result<Option<Value>, Error> {
        Ok(None)
    }
}
impl Command {
    pub async fn respond(&mut self, stream: &mut TcpStream) {

    }
}

// impl RedisCommand for Command {
//     fn execute(&mut self, storage: &mut Storage) -> Result<Option<Value>, Error> {
//         match self {
//             Command::PING => {
//                 stream.write_buf(&mut Value::SimpleString("PONG".to_string()).encode()).await.expect("Could not send pong");
//             },
//             Command::ECHO(arg) => {
//                 eprintln!("responding to command ECHO with argument {:?}", arg);
//
//                 stream.write_buf(&mut arg.encode()).await.expect("Could not send pong");
//             }
//             _ => {}
//         }
//     }
// }