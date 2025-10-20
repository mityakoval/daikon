use crate::resp_parser::parser::parse_command;
use std::fmt::Display;
use std::io::{Read, Write};
use std::net::TcpStream;

mod resp_parser;

enum Command<'a> {
    PING,
    ECHO(Value<'a>),
}

impl Command<'_> {
    fn respond(&mut self, mut stream: &TcpStream) {
        match self {
            Command::PING => stream
                .write_all(Value::SimpleString("PONG").encode().as_slice())
                .unwrap(),
            Command::ECHO(arg) => {
                eprintln!("responding to command ECHO with argument {:?}", arg);

                stream.write_all(arg.encode().as_slice()).unwrap()
            }
        }
    }
}

trait RESPType {
    fn encode(&mut self) -> Vec<u8>;
}

#[derive(Debug)]
enum Value<'a> {
    Array(Vec<Value<'a>>),
    SimpleString(&'a str),
    BulkString(&'a str),
    NullBulkString(),
    Error(&'a str),
}

impl<'a> RESPType for Value<'a> {
    fn encode(&mut self) -> Vec<u8> {
        let mut encoded: Vec<u8> = Vec::new();
        match self {
            Value::Array(array) => {
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .into_iter()
                    .flat_map(|t| t.encode())
                    .for_each(|c| encoded.push(c));
            }
            Value::SimpleString(value) => {
                encoded.extend_from_slice(format!("+{}\r\n", value).as_bytes());
            }
            Value::BulkString(value) => {
                encoded.extend_from_slice(format!("${}\r\n{}\r\n", value.len(), value).as_bytes());
            }
            Value::NullBulkString() => {
                encoded.extend_from_slice(b"$-1\r\n");
            }
            Value::Error(msg) => {
                encoded.extend_from_slice(format!("-{}\r\n", msg).as_bytes());
            }
        };
        encoded
    }
}

pub fn handle_connection(mut stream: &TcpStream) {
    let mut buf = [0; 512];
    loop {
        match stream.read(&mut buf) {
            Ok(size) => {
                if size == 0 {
                    break;
                }

                match str::from_utf8(&buf) {
                    Ok(input) => {
                        let mut command = parse_command(input).unwrap();
                        command.respond(&stream);
                    }
                    Err(_e) => {}
                }
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                break;
            }
        }
    }
}
