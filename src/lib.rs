use std::fmt::Display;
use crate::resp_parser::parser::parse_command;
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
            Command::PING => stream.write_all(b"+PONG\r\n").unwrap(),
            Command::ECHO(arg) => stream.write_all(arg.encode().as_slice()).unwrap(),
        }
    }
}

trait RESPType {
    fn encode(&mut self) -> Vec<u8>;
    
    fn len(&self) -> usize;
}

#[derive(Debug)]
enum Value<'a> {
    Array(Vec<Value<'a>>),
    SimpleString(&'a str),
    BulkString(&'a str),
    Error(&'a str),
}

impl<'a> RESPType for Value<'a> {
    fn encode(&mut self) -> Vec<u8> {
        match self {
            Value::Array(array) => {
                let mut encoded: Vec<u8> = Vec::new();
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .into_iter()
                    .flat_map(|t| t.encode())
                    .for_each(|c| encoded.push(c));

                encoded
            }
            Value::SimpleString(value) => value.as_bytes().to_vec(),
            Value::BulkString(value) => value.as_bytes().to_vec(),
            Value::Error(msg) => msg.as_bytes().to_vec(),
        }
    }
    
    fn len(&self) -> usize {
        match self {
            Value::Array(values) => { values.len() }
            Value::SimpleString(value) => { value.len() }
            Value::BulkString(value) => { value.len() }
            Value::Error(value) => { value.len() }
        }
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
