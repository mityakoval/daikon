use clap::Parser;
use daikon::handle_connection;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// port to bind to
    #[arg(short, long, default_value_t = 6379)]
    port: u16,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let port = args.port;

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .expect("Failed to bind");

    let storage_mutex_arc = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, _socket_addr) = listener.accept().await.unwrap();
        tokio::spawn(handle_connection(stream, Arc::clone(&storage_mutex_arc)));
    }
}
