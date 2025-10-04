#![allow(unused_imports)]

use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    loop {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0; 512];
                    loop {
                        let command = stream.read(&mut buf).unwrap();

                        if command == 0 {
                            break;
                        }

                        stream.write_all(b"+PONG\r\n").unwrap()
                    }
                }
                Err(e) => {
                    println!("error: {}", e);
                }
            }
        }
    }
}
