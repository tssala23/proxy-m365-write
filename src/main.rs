use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use proxy_m365_write::{AppState, config::Config, handle};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load()?;
    let listen_addr: SocketAddr = config.listen_addr.parse()?;
    let state = Arc::new(AppState::new(config));
    let listener = TcpListener::bind(listen_addr).await?;
    println!("listening on http://{listen_addr}");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(error) => {
                eprintln!("[ERROR] accept failed: {error}");
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let service = service_fn(move |request| handle(Arc::clone(&state), request));
            if let Err(error) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("[ERROR] connection error: {error}");
            }
        });
    }
}
