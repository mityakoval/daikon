#![allow(unused_imports)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Reading a stream might block the thread
                tokio::task::spawn_blocking(move || handle_connection(stream));
            }
            Err(e) => {
                eprintln!("Error obtaining stream: {}", e)
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0; 512];
    loop {
        match stream.read(&mut buf) {
            Ok(size) => {
                if size == 0 {
                    break;
                }

                stream.write_all(b"+PONG\r\n").unwrap()
            }
            Err(e) => {
                eprintln!("Error reading stream: {}", e)
            }
        }
    }
}

// fn parse_command(string: String) -> Commands {
// }
