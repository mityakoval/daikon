use std::time::Duration;
use std::vec::IntoIter;
use anyhow::{anyhow, Error};
use bytes::BytesMut;
use crate::data::commands::Command;
use crate::data::types::Value;
use crate::parser::input::parse_data_bytes;

pub(crate) fn parse_command(input: &mut BytesMut) -> anyhow::Result<Command> {
    match parse_data_bytes(input).unwrap() {
        (Value::Array(command_array), _) => {
            eprintln!("command array: {:?}", command_array);
            let mut command_array = command_array.into_iter();

            if let Some(Value::BulkString(command_bulk_str)) = command_array.next() {
                match command_bulk_str.to_uppercase().as_str() {
                    "PING" => Ok(Command::PING),
                    "ECHO" => {
                        let arg = command_array.next().unwrap();
                        eprintln!("arg: {:?}", arg);
                        Ok(Command::ECHO(arg))
                    }
                    "SET" => parse_set(command_array),
                    "GET" => {
                        match command_array.next().unwrap() {
                            Value::BulkString(key) => {
                                Ok(Command::GET(key))
                            }
                            _ => Err(Error::msg("Wrong type"))
                        }
                    }
                    _ => Err(Error::msg("Unknown command")),
                }
            } else {
                Err(anyhow!("command array empty or malformed"))
            }
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
            let ttl = if let Some(Value::BulkString(duration)) = command_array.next_if(|v| *v == Value::BulkString("EX".into()) || *v == Value::BulkString("PX".into())) {
                let ttl = match command_array.next() {
                    Some(Value::BulkString(ttl)) => ttl.parse::<u64>()?,
                    _ => {
                        return Err(anyhow!("Malformed command"));
                    }
                };
                match duration.to_uppercase().as_str() {
                    "EX" => {
                        Some(Duration::from_secs(ttl))
                    }
                    "PX" => {
                        Some(Duration::from_millis(ttl))
                    }
                    _ => return Err(anyhow!("Wrong syntax")),
                }
            } else {
                None
            };

            Ok(Command::SET {
                key,
                value,
                ttl,
            })
        }
        _ => Err(Error::msg("Wrong type"))
    }
}