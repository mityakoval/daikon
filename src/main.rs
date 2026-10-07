use codecrafters_redis::handle_connection;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .expect("Failed to bind");

    let storage = Arc::new(DashMap::new());

    loop {
        let (stream, _socket_addr) = listener.accept().await.unwrap();
        tokio::spawn(handle_connection(stream, Arc::clone(&storage)));
    }
}
