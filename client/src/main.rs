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

use crate::socks5::session::ConnectTarget;
use crate::tun::DnsResolver;
use crate::tun::FakeDns;
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

            if session.handshake().await? == consts::s5::auth::AUTH { session.auth().await?; }
            session.connect(ConnectTarget::FromConfig).await?;
            session.send().await
        },
        Mode::Tun2Socks => {
            let cancel_token = CancellationToken::new();

            let dns_resolver = if config.fake_dns { Some(DnsResolver::new()) } else { None };

            let mut tasks = Vec::new(); 

            let mut session = TunSession::new(config.clone(), dns_resolver.clone(), cancel_token.clone())?;

            tasks.push(tokio::spawn(async move { session.run().await; }));

            if let Some(dns_resolver) = dns_resolver {
                let mut fake_dns = FakeDns::new(&config, dns_resolver.clone(), cancel_token.clone()).await?;
                tasks.push(tokio::spawn(async move { fake_dns.run().await; }));
            }
            
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down...");
                    cancel_token.cancel();
                    for task in tasks { let _ = task.await; }
                }
                result = async {
                    futures::future::select_all(&mut tasks).await
                } => {
                    if let Err(error) = result.0 {
                        error!(%error, "task crashed");
                        cancel_token.cancel();
                    }
                    for task in tasks { let _ = task.await; }
                }
            }

            Ok(())
        },
        Mode::Tun => Err(AppError::Tun(format!("mode {:?} not yet implemented", config.mode))),
    }
}

#[cfg(debug_assertions)]
fn setup_tracing() {
    fmt()
        .with_target(true)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("rustls=warn,s5t=trace,s5l=trace"))
        )
        .init();
}

#[cfg(not(debug_assertions))]
fn setup_tracing() {
    let mut directives = vec![
        "smoltcp=error".to_string(),
        "netstack_smoltcp=error".to_string(),
        "rustls=error".to_string(),
    ];

    if let Ok(env_filter) = EnvFilter::try_from_default_env() {
        directives.push(env_filter.to_string());
    } else {
        directives.push("s5t=info".to_string());
    }

    let filter = EnvFilter::new(directives.join(","));

    fmt()
        .with_target(false)
        .with_env_filter(filter)
        .init();
}