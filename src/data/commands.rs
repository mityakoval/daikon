use crate::data::types::Value;
use std::time::Duration;

pub enum Command {
    PING,
    ECHO(Value),
    SET {
        key: String,
        value: Value,
        ttl: Option<Duration>
    },
    GET(String),
}
