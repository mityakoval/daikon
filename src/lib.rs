use crate::resp_parser::parser::parse_command;
use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

mod resp_parser;

enum Command<> {
    PING,
    ECHO(Value<>),
}

impl Command {
    async fn respond(&mut self, stream: &mut TcpStream) {
        match self {
            Command::PING => {
                stream.write_buf(&mut Value::SimpleString("PONG".to_string()).encode()).await.expect("Could not send pong");
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
enum Value {
    Array(Vec<Value>),
    SimpleString(String),
    BulkString(String),
    NullBulkString(),
}

struct Error<'a> {
    msg: &'a str,
}

impl RESPType for Value {
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
        };
        encoded
    }
}

pub async fn handle_connection(mut stream: TcpStream) {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        match stream.read_buf(&mut buf).await {
            Ok(size) => {
                if size == 0 {
                    break;
                }

                match parse_command(&mut buf) {
                    Ok(mut command) => {
                        command.respond(&mut stream).await;
                    }
                    Err(e) => {
                        eprintln!("Error parsing command: {:#}", e);
                        stream.write_all("-Unknown command\r\n".as_bytes()).await.expect("Could not send error");
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
