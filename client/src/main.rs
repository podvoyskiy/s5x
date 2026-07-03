#![warn(clippy::pedantic)]

mod prelude;
mod mode;
mod config;
mod socks5;
mod tun;
mod http;

use prelude::*;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;

use crate::{tun::TunSession, socks5::Socks5Session};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    setup_tracing();

    let mut config = Config::new()?;
    config.validate()?;

    debug!(config = ?config, "client started");

    match config.mode {
        Mode::Socks5 => {
            let stream = TcpStream::connect(config.server).await.map_err(|_| AppError::TargetUnreachable)?;
            let mut session = Socks5Session::new(config, stream);

            if session.handshake().await? == consts::auth::AUTH { session.auth().await?; }
            session.connect().await?;
            session.send().await
        },
        Mode::Tun => {
            let cancel_token = CancellationToken::new();
            let mut session = TunSession::new(&config, cancel_token.clone())?;

            let handle = tokio::task::spawn_blocking(move || {
                session.run();
            });

            tokio::signal::ctrl_c().await?;
            cancel_token.cancel();
            let _ = handle.await;
            Ok(())
        },
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