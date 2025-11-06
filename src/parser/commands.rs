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
                    "ECHO" => {
                        let arg = command_array.next().unwrap();
                        eprintln!("arg: {:?}", arg);
                        Ok(Command::ECHO(arg))
                    }
                    "PING" => Ok(Command::PING),
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

fn parse_set(mut command_array: IntoIter<Value>) -> anyhow::Result<Command> {
    let key = command_array.next().unwrap();
    match key {
        Value::BulkString(key) => {
            let value = command_array.next().unwrap();
            Ok(Command::SET {
                key,
                value,
                ttl: None
            })
        }
        _ => Err(Error::msg("Wrong type"))
    }
}