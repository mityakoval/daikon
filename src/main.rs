use std::net::TcpListener;
use codecrafters_redis::handle_connection;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Reading a stream might block the thread
                tokio::task::spawn_blocking(move || handle_connection(&stream));
            }
            Err(e) => {
                eprintln!("Error obtaining stream: {}", e)
            }
        }
    }
}
