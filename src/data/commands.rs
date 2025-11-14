use std::str::FromStr;
use crate::data::types::Value;
use std::time::Duration;

pub enum Command {
    PING,
    ECHO,
    SET,
    GET,
    RPUSH,
}

impl FromStr for Command {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PING" => Ok(Command::PING),
            "ECHO" => Ok(Command::ECHO),
            "SET" => Ok(Command::SET),
            "GET" => Ok(Command::GET),
            "RPUSH" => Ok(Command::RPUSH),
            _ => Err(()),
        }
    }
}

pub struct CommandArray {
    pub(crate) command: Command,
    length: u8,
    key: Option<String>,
    value: Option<Value>,
    ttl: Option<Duration>
}