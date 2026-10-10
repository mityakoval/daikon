use anyhow::anyhow;

use crate::data::resp_types::RESPValue;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub(crate) enum ValueType {
    None,
    Single,
    List,
}

#[derive(Debug)]
pub(crate) struct CommandRules {
    pub(crate) has_key: bool,
    pub(crate) allows_ttl: bool,
    pub(crate) value_type: ValueType,
}

#[derive(Debug)]
pub enum CommandName {
    Ping,
    Echo,
    Set,
    Get,
    RPush,
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
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PING" => Ok(Command {
                name: CommandName::Ping,
                rules: CommandRules {
                    has_key: false,
                    allows_ttl: false,
                    value_type: ValueType::None,
                },
            }),
            "ECHO" => Ok(Command {
                name: CommandName::Echo,
                rules: CommandRules {
                    allows_ttl: false,
                    has_key: false,
                    value_type: ValueType::Single,
                },
            }),
            "SET" => Ok(Command {
                name: CommandName::Set,
                rules: CommandRules {
                    allows_ttl: true,
                    has_key: true,
                    value_type: ValueType::Single,
                },
            }),
            "GET" => Ok(Command {
                name: CommandName::Get,
                rules: CommandRules {
                    allows_ttl: false,
                    has_key: true,
                    value_type: ValueType::None,
                },
            }),
            "RPUSH" => Ok(Command {
                name: CommandName::RPush,
                rules: CommandRules {
                    has_key: true,
                    allows_ttl: true,
                    value_type: ValueType::List,
                },
            }),
            _ => Err(anyhow!("Unknown command '{}'", s)),
        }
    }
}

#[derive(Debug)]
pub struct CommandArray {
    pub(crate) command: Command,
    pub(crate) key: Option<Arc<str>>,
    pub(crate) value: Option<Vec<RESPValue>>,
    pub(crate) ttl: Option<Duration>,
}
