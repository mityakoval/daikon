use crate::data::errors::ParsingError::{self, InvalidCommand};
use crate::data::resp_types::RespValue;
use std::collections::VecDeque;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub(crate) enum Kommand {
    Ping,
    Echo(RespValue),
    Set {
        key: Arc<str>,
        value: RespValue,
        ttl: Duration,
    },
    Get(Arc<str>),
    Del(Arc<str>),
    Rpush {
        key: Arc<str>,
        values: VecDeque<RespValue>,
        ttl: Duration,
    },
}

#[derive(Debug)]
pub(crate) enum ArgShape {
    None,
    Value,
    Key,
    MultipleKeys,
    KeyValue,
    KeyMultipleValues,
    Pairs,
}

#[derive(Debug)]
pub(crate) enum KeyType {
    None,
    Single,
    List,
}

#[derive(Debug)]
pub(crate) struct CommandRules {
    pub(crate) allows_ttl: bool,
    pub(crate) args: ArgShape,
}

#[derive(Debug)]
pub enum CommandName {
    Ping,
    Echo,
    Set,
    Get,
    Del,
    Rpush,
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

#[derive(Debug)]
pub(crate) struct Command {
    pub(crate) name: CommandName,
    pub(crate) rules: CommandRules,
}

impl FromStr for Command {
    type Err = ParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PING" => Ok(Command {
                name: CommandName::Ping,
                rules: CommandRules {
                    allows_ttl: false,
                    args: ArgShape::None,
                },
            }),
            "ECHO" => Ok(Command {
                name: CommandName::Echo,
                rules: CommandRules {
                    allows_ttl: false,
                    args: ArgShape::Value,
                },
            }),
            "SET" => Ok(Command {
                name: CommandName::Set,
                rules: CommandRules {
                    allows_ttl: true,
                    args: ArgShape::KeyValue,
                },
            }),
            "GET" => Ok(Command {
                name: CommandName::Get,
                rules: CommandRules {
                    allows_ttl: false,
                    args: ArgShape::Key,
                },
            }),
            "DEL" => Ok(Command {
                name: CommandName::Del,
                rules: CommandRules {
                    allows_ttl: false,
                    args: ArgShape::MultipleKeys,
                },
            }),
            "RPUSH" => Ok(Command {
                name: CommandName::Rpush,
                rules: CommandRules {
                    allows_ttl: true,
                    args: ArgShape::KeyMultipleValues,
                },
            }),
            _ => Err(InvalidCommand(format!("Unknown command '{}'", s))),
        }
    }
}

#[derive(Debug)]
pub struct CommandArray {
    pub(crate) command: Command,
    pub(crate) keys: Option<Vec<Arc<str>>>,
    pub(crate) values: Option<VecDeque<RespValue>>,
    pub(crate) ttl: Option<Duration>,
}
