use std::cmp::PartialEq;
use std::str::FromStr;
use crate::data::commands::{Command, CommandArray};
use crate::data::types::Value;
use crate::parser::input::parse_data_bytes;
use anyhow::{anyhow, Error};
use bytes::BytesMut;
use std::time::Duration;
use std::vec::IntoIter;

pub(crate) fn parse_command_array(input: &mut BytesMut) -> anyhow::Result<CommandArray> {
    match parse_data_bytes(input).unwrap() {
        (Value::Array(command_array), _) => {
            eprintln!("command array: {:?}", command_array);
            let mut command_array = command_array.into_iter();
            let mut command: Command;

            return if let Some(Value::BulkString(command_bulk_str)) = command_array.next() {
                command = Command::from_str(command_bulk_str.to_uppercase().as_str()).or_else(|_| Err(anyhow!("Error parsing command bulk string")))?;
                if matches!(command, Command::SET) || matches!(command, Command::GET) || matches!(command, Command::RPUSH) {

                }
                Ok(CommandArray {
                    command,
                    length: 1
                })
            } else {
                Err(anyhow!("command array empty or malformed"))
            };

        }
        _ => Err(Error::msg("Command must be a RESP array")),
    }
}

fn parse_set(command_array: IntoIter<Value>) -> anyhow::Result<Command> {
    let mut command_array = command_array.peekable();
    let key = command_array.next().unwrap();
    match key {
        Value::BulkString(key) => {
            let value = command_array.next().unwrap();
            let ttl = if let Some(Value::BulkString(duration)) = command_array.next_if(|v| {
                *v == Value::BulkString("EX".into()) || *v == Value::BulkString("PX".into())
            }) {
                let ttl = match command_array.next() {
                    Some(Value::BulkString(ttl)) => ttl.parse::<u64>()?,
                    _ => {
                        return Err(anyhow!("Malformed command"));
                    }
                };
                match duration.to_uppercase().as_str() {
                    "EX" => Some(Duration::from_secs(ttl)),
                    "PX" => Some(Duration::from_millis(ttl)),
                    _ => return Err(anyhow!("Wrong syntax")),
                }
            } else {
                None
            };

            Ok(Command::SET { key, value, ttl })
        }
        _ => Err(Error::msg("Wrong type")),
    }
}
