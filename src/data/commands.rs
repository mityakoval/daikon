use crate::data::errors::ParsingError;
use crate::data::resp_types::RespValue;
use std::collections::VecDeque;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub(crate) enum Command {
    Ping,
    Echo(RespValue),
    Set {
        key: Arc<str>,
        value: RespValue,
        ttl: Option<Duration>,
    },
    Get(Arc<str>),
    Del(Vec<Arc<str>>),
    Rpush {
        key: Arc<str>,
        values: VecDeque<RespValue>,
    },
    Lrange {
        key: Arc<str>,
        start: i16,
        stop: i16,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Ping => CommandName::Ping.fmt(f),
            Command::Echo(_) => CommandName::Echo.fmt(f),
            Command::Set {
                key: _,
                value: _,
                ttl: _,
            } => CommandName::Set.fmt(f),
            Command::Get(_) => CommandName::Get.fmt(f),
            Command::Del(_) => CommandName::Del.fmt(f),
            Command::Rpush { key: _, values: _ } => CommandName::Rpush.fmt(f),
            Command::Lrange {
                key: _,
                start: _,
                stop: _,
            } => CommandName::Lrange.fmt(f),
        }
    }
}

#[derive(Debug)]
pub enum CommandName {
    Ping,
    Echo,
    Set,
    Get,
    Del,
    Rpush,
    Lrange,
}
impl FromStr for CommandName {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PING" => Ok(CommandName::Ping),
            "ECHO" => Ok(CommandName::Echo),
            "SET" => Ok(CommandName::Set),
            "GET" => Ok(CommandName::Get),
            "DEL" => Ok(CommandName::Del),
            "RPUSH" => Ok(CommandName::Rpush),
            _ => Err(ParsingError::UnknownCommand),
        }
    }
}

impl Display for CommandName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}
