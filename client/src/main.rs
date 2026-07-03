#![warn(clippy::pedantic)]

mod prelude;
mod mode;
mod socks5;
mod http;

use prelude::*;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;

use crate::socks5::{config::Socks5Config, session::Socks5Session};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    setup_tracing();

    let mut config = Socks5Config::new()?;
    config.validate()?;

    match config.mode {
        Mode::Cli => {
            debug!(config = ?config, "socks5 client started");
            let stream = TcpStream::connect(config.server).await.map_err(|_| AppError::TargetUnreachable)?;
            let mut session = Socks5Session::new(config, stream);

            if session.handshake().await? == consts::auth::AUTH { session.auth().await?; }
            session.connect().await?;
            session.send().await
        },
        Mode::Proxy => {
            let listener = TcpListener::bind(config.listen).await?;
            info!(config = ?config, "socks5 client started");

            loop {
                let (mut client_stream, client_addr) = listener.accept().await?;
                info!(%client_addr, "new connection");

                tokio::spawn(async move {
                    let mut server_stream = TcpStream::connect(config.server).await.map_err(|_| AppError::TargetUnreachable)?;
                    tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await?;
                    info!(%client_addr, "connection closed");
                    Ok::<(), AppError>(())
                });
            }
        },
        Mode::_Tun => Err(AppError::Socks5(format!("mode {:?} not yet implemented", config.mode))),
    }
}

#[cfg(debug_assertions)]
fn setup_tracing() {
    fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("rustls=warn,s5t=trace,s5l=trace"))
        )
        .init();
}

#[cfg(not(debug_assertions))]
fn setup_tracing() {
    fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("rustls=error,s5t=info"))
        )
        .init();
}