use crate::data::commands::{Command, CommandArray};
use crate::data::types::Value;
use crate::parser::input::parse_data_bytes;
use anyhow::{anyhow, Error};
use bytes::BytesMut;
use std::str::FromStr;
use std::time::Duration;
use std::vec::IntoIter;

pub(crate) fn parse_command_array(input: &mut BytesMut) -> anyhow::Result<CommandArray> {
    match parse_data_bytes(input).unwrap() {
        (Value::Array(command_array), _) => {
            eprintln!("command array: {:?}", command_array);
            let mut command_array = command_array.into_iter();
            let command: Command;

            if let Some(Value::BulkString(command_bulk_str)) = command_array.next() {
                command = Command::from_str(command_bulk_str.to_uppercase().as_str())
                    .map_err(|_| anyhow!("Error parsing command bulk string"))?;
                let mut key = None;
                let mut value = None;
                let mut ttl = None;
                match command {
                    Command::Ping => value = Some(Value::SimpleString("PONG".into())),

                    Command::Echo => {
                        value = Some(Value::SimpleString(parse_key(command_array)?.0));
                    }

                    Command::Set => {
                        let (_key, rest) = parse_key(command_array)?;
                        key = Some(_key);
                        let (_value, rest) = parse_value(rest)?;
                        value = Some(_value);
                        ttl = parse_ttl(rest);
                    }

                    Command::Get => {
                        let (_key, _) = parse_key(command_array)?;
                        key = Some(_key);
                    }

                    Command::RPush => {}
                }
                Ok(CommandArray {
                    command,
                    key,
                    value,
                    ttl,
                })
            } else {
                Err(anyhow!("command array empty or malformed"))
            }
        }
        _ => Err(Error::msg("Command must be a RESP array")),
    }
}

fn parse_key(mut command_array: IntoIter<Value>) -> anyhow::Result<(String, IntoIter<Value>)> {
    match command_array.next() {
        Some(Value::BulkString(key)) => Ok((key, command_array)),
        _ => Err(anyhow!("key must be a BulkString")),
    }
}

fn parse_value(mut command_array: IntoIter<Value>) -> anyhow::Result<(Value, IntoIter<Value>)> {
    match command_array.next() {
        Some(Value::BulkString(value)) => Ok((Value::BulkString(value), command_array)),
        _ => Err(anyhow!("Couldn't parse value")),
    }
}

fn parse_ttl(command_array: IntoIter<Value>) -> Option<Duration> {
    let arr = command_array
        .filter_map(|v| {
            if let Value::BulkString(value) = v {
                Some(value)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if arr.len() < 2 {
        return None;
    }
    if let Ok(ttl) = arr[1].parse::<u64>() {
        let ttl_str = arr[0].to_uppercase();
        if ttl_str == "EX" {
            Some(Duration::from_secs(ttl))
        } else if ttl_str == "PX" {
            Some(Duration::from_millis(ttl))
        } else {
            None
        }
    } else {
        None
    }
}
