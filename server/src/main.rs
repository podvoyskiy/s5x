#![warn(clippy::pedantic)]

mod socks5;
mod prelude;

use prelude::*;
use tokio::net::TcpListener;
use tracing_subscriber::fmt;
use crate::socks5::{config::Config, session::Socks5Session};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    setup_tracing();

    let mut config = Config::new()?;
    config.validate()?;

    let listener = TcpListener::bind(format!("{}:{}", config.host, config.port)).await
        .map_err(|_| AppError::Socks5(format!("port {} is busy or cannot be used", config.port)))?;

    info!(config = ?config, "socks5 started");

    loop {
        let config = config.clone();
        let (stream, client_addr) = listener.accept().await?;
        info!(%client_addr, "new connection");
        tokio::spawn(async move {
            let mut session = Socks5Session::new(config, stream);
            if let Err(error) = session.start().await {
                error!(%error, %client_addr);
            }
            info!(%client_addr, "connection closed");
        });
    }
}

#[cfg(debug_assertions)]
fn setup_tracing() {
    use tracing::Level;

    fmt()
        .with_target(false)
        .with_max_level(Level::TRACE)
        .init();
}

#[cfg(not(debug_assertions))]
fn setup_tracing() {
    use tracing_subscriber::EnvFilter;

    fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("s5x=info"))
        )
        .init();
}