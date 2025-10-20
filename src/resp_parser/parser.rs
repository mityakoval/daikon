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
                        ("ECHO") => {
                            let arg = command_array.next().unwrap();
                            eprintln!("arg: {:?}", arg);
                            Ok(Command::ECHO(arg))
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
        eprintln!("parsing chunk: {}, rest: {}", chunk, rest);
        let mut chars = chunk.chars();
        match chars.next() {
            Some('$') => {
                // It's a BulkString
                if let Ok(length) = chars.collect::<String>().parse::<usize>()  {
                    let content = &rest[..(length)];
                    eprintln!("got bulk string {:?} of length: {}", content, length);
                    let new_rest = &rest[(length)+2..];
                    eprintln!("new rest is {}", new_rest);
                    Some((Value::BulkString(content), new_rest))
                } else {
                    None
                }
            }
            Some('*') => {
                // It's a RESPArray
                if let Ok(length) = chars.collect::<String>().parse::<usize>() {
                    eprintln!("got a resp array of length: {}", length);
                    let mut array = Vec::with_capacity(length);
                    let mut rest = rest;
                    for i in 0..length {
                        eprintln!("parsing item {}", i);
                        let (t, new_rest) = parse_data(rest)?;
                        array.push(t);
                        rest = new_rest;
                    }
                    Some((Value::Array(array), rest))
                } else {
                    None
                }
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
