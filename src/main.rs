use codecrafters_redis::handle_connection;
use std::error::Error;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await.expect("Failed to bind");

    match listener.accept().await {
        Ok((stream, _socket_addr)) => {
            tokio::spawn(|| async move {
                handle_connection(stream);
            });
            Ok(())
        }
        Err(e) => {
           Err(Box::new(e)).unwrap()
        }
    }
}
