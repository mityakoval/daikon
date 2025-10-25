use std::sync::Arc;
use codecrafters_redis::handle_connection;
use tokio::net::TcpListener;
use codecrafters_redis::storage::Storage;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.expect("Failed to bind");

    let storage = Arc::new(Storage::new());

    loop {
        let (stream, _socket_addr) = listener.accept().await.unwrap();
        handle_connection(stream, Arc::clone(&storage)).await;
    }
}
