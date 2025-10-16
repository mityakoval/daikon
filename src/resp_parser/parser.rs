use crate::{Command, RESPType, Value};
use anyhow::Error;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

pub fn parse_command(input: &str) -> anyhow::Result<Command<'_ >> {
    match parse_data(input).unwrap() {
        (Value::Array(command_array), _) => {
            eprintln!("command array: {:?}", command_array);
            let mut command_array = command_array.into_iter();
            match command_array.next().unwrap() {
                Value::BulkString(command_bulk_str) => {
                    eprintln!("command: {}", command_bulk_str);
                    match command_bulk_str {
                        "ECHO" => {
                            let arg = command_array.next().unwrap();

                            Ok(Command::ECHO(command_array.next().unwrap()))
                        }
                        "PING" => {
                            Ok(Command::PING)
                        }
                        _ => { Err(Error::msg("Unknown command")) }
                    }
                }
                _ => { Err(Error::msg("Command must be a RESP array")) }
            }
        },
        _ => Err(Error::msg("Command must be a RESP array")),
    }
}

pub fn parse_data(input: &str) -> Option<(Value<'_>, &str)> {
    if let Some((chunk, rest)) = input.split_once("\r\n") {
        eprintln!("parsing chunk: {}, rest: {:?}", chunk, rest.trim());
        let mut chars = chunk.chars();
        match chars.next() {
            Some('$') => {
                // It's a BulkString
                if let Some(length) = chars.next().unwrap().to_digit(10) {
                    eprintln!("got bulk string of length: {}", length);
                    Some((Value::BulkString(&rest[..(length as usize)]), rest))
                } else {
                    None
                }
            }
            Some('*') => {
                // It's a RESPArray
                let length = chunk.chars().next().unwrap().len_utf8();
                let mut array = Vec::with_capacity(length);
                let mut rest = rest;
                for i in 0..length {
                    let (t, new_rest) = parse_data(rest)?;
                    array.push(t);
                    rest = new_rest;
                }
                Some((Value::Array(array), rest))
            }
            _ => None,
        }
    } else {
        None
    }
}

#[derive(Error, Debug)]
enum ParseError {
    UnexpectedInput(),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
