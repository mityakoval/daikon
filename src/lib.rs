use crate::resp_parser::parser::parse;
use std::io::{Read, Write};
use std::net::TcpStream;

mod resp_parser;

enum Command {
    PING,
    ECHO(String),
}

impl Command {
    fn respond(&self, mut stream: &TcpStream) {
        match self {
            Command::PING => stream.write_all(b"+PONG\r\n").unwrap(),
            Command::ECHO(arg) => {
                stream.write_all(arg.as_bytes()).unwrap()
            },
        }
    }
}

enum RESPType {
    SimpleString(String),
    BulkString(String),
    Error(String)
}

impl RESPType {
    
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
                        let command = parse(input).unwrap();
                        command.respond(&stream);
                    }
                    Err(e) => {

                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                break;
            }
        }
    }
}
