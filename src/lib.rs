use std::error::Error;
use crate::resp_parser::parser::parse_command;
use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

mod resp_parser;

enum Command<'a> {
    PING,
    ECHO(Value<'a>),
}

impl Command<'_> {
    async fn respond(&mut self, stream: &mut TcpStream) {
        match self {
            Command::PING => {
                stream.write_buf(&mut Value::SimpleString("PONG").encode()).await.expect("Could not send pong");
            },
            Command::ECHO(arg) => {
                eprintln!("responding to command ECHO with argument {:?}", arg);

                stream.write_buf(&mut arg.encode()).await.expect("Could not send pong");
            }
        }
    }
}

trait RESPType {
    fn encode(&mut self) -> BytesMut;
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
    fn encode(&mut self) -> BytesMut {
        let mut encoded: BytesMut = BytesMut::new();
        match self {
            Value::Array(array) => {
                // Prepend with the array length
                encoded.extend_from_slice(format!("*{}\r\n", array.len()).as_bytes());

                array
                    .into_iter()
                    .flat_map(|t| t.encode())
                    .for_each(|c| encoded.put_u8(c));
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

pub async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        match stream.read_buf(&mut buf).await {
            Ok(size) => {
                if size == 0 {
                    break Ok(());
                }

                match str::from_utf8(&buf) {
                    Ok(input) => {
                        let mut command = parse_command(input)?;
                        command.respond(&mut stream).await;
                    }
                    Err(_e) => {}
                }
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e);
                break Err(Box::new(e));
            }
        }
    }
}
