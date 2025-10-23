use codecrafters_redis::handle_connection;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.expect("Failed to bind");


    loop {
        let (stream, _socket_addr) = listener.accept().await.unwrap();
        handle_connection(stream).await;
    }
}
