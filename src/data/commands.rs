use crate::data::types::Value;
use std::str::FromStr;
use std::time::Duration;

pub enum Command {
    Ping,
    Echo,
    Set,
    Get,
    RPush,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PING" => Ok(Command::Ping),
            "ECHO" => Ok(Command::Echo),
            "SET" => Ok(Command::Set),
            "GET" => Ok(Command::Get),
            "RPUSH" => Ok(Command::RPush),
            _ => Err(()),
        }
    }
}

pub struct CommandArray {
    pub(crate) command: Command,
    pub(crate) key: Option<String>,
    pub(crate) value: Option<Value>,
    pub(crate) ttl: Option<Duration>,
}
