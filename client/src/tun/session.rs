use std::net::{IpAddr, Ipv4Addr};
use netstack_smoltcp::{Stack, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;
use netstack_smoltcp::TcpStream as NetstackTcpStream;
use tokio_util::sync::CancellationToken;
use tun::{AbstractDevice, AsyncDevice};
use crate::prelude::*;
use crate::socks5::Socks5Session;
use crate::socks5::session::ConnectTarget;
use crate::tun::{DnsResolver, Routing};
use futures_util::{SinkExt, stream::{SplitSink, SplitStream, StreamExt}};

pub struct TunSession {
    config: Config,
    resolver: Option<DnsResolver>,
    cancel_token: CancellationToken,
    dev: Option<AsyncDevice>,
    routing: Routing,

    tcp_listener: TcpListener,
    stack_sink: Option<SplitSink<Stack, Vec<u8>>>,
    stack_stream: Option<SplitStream<Stack>>,
}

impl TunSession {
    pub fn new(config: Config, resolver: Option<DnsResolver>, cancel_token: CancellationToken) -> Result<Self, AppError> {
        let destination = Ipv4Addr::new(
            config.address.octets()[0], 
            config.address.octets()[1], 
            config.address.octets()[2], 
            1
        );
            
        let mut tun_config = tun::Configuration::default();
        tun_config
            .address(config.address)
            .netmask((255, 255, 255, 0))
            .destination(destination)
            .up();
        
        #[cfg(target_os = "linux")]
        tun_config.platform_config(|config: &mut tun::PlatformConfig| { config.ensure_root_privileges(true); });

        let dev = tun::create_as_async(&tun_config)
            .map_err(|e| AppError::Tun(format!("failed to create tun interface: {e}")))?;
        
        let tun_index: u32 = dev.tun_index().map_err(|e| AppError::Tun(format!("{e}")))?.try_into()?;

        let routing = Routing::new(&config, tun_index)?;
        routing.setup()?;

        let (stack, runner, _, tcp_listener) = netstack_smoltcp::StackBuilder::default()
            .stack_buffer_size(consts::buffer::STACK_PACKET)
            .tcp_buffer_size(consts::buffer::TCP_STACK)
            .enable_udp(false)
            .enable_tcp(true)
            .enable_icmp(true)
            .mtu(consts::buffer::VIRTUAL_MTU)
            .build()
            .map_err(|e| AppError::Tun(format!("failed to build netstack: {e}")))?;

        if let Some(runner) = runner {
            tokio::spawn(runner);
        }

        let tcp_listener = tcp_listener.ok_or_else(|| AppError::Tun("tcp not enabled".to_string()))?;
        let (stack_sink, stack_stream) = stack.split();
        
        Ok(Self { 
            config, 
            resolver, 
            cancel_token, 
            dev: Some(dev), 
            routing,
            tcp_listener,
            stack_sink: Some(stack_sink),
            stack_stream: Some(stack_stream)
        })
    }

    pub async fn run(&mut self) {
        let framed = self.dev.take().unwrap().into_framed();

        let (mut tun_sink, mut tun_stream) = framed.split();
        let mut stack_sink = self.stack_sink.take().unwrap();
        let mut stack_stream = self.stack_stream.take().unwrap();
        let cancel_token = self.cancel_token.clone();

        //* read packets from stack and send to tun
        let mut stack_to_tun = tokio::spawn(async move {
            while let Some(pkt) = stack_stream.next().await {
                if let Ok(pkt) = pkt && let Err(error) = tun_sink.send(pkt).await {
                    error!(%error, "failed to send to tun");
                    break;
                }
            }
        });

        //* read packets from tun and send to stack
        let mut tun_to_stack = tokio::spawn(async move {
            while let Some(pkt) = tun_stream.next().await {
                if let Ok(pkt) = pkt && let Err(error) = stack_sink.send(pkt).await {
                    error!(%error, "failed to send to stack");
                    break;
                }
            }
        });

        let tcp_handler = self.handle_tcp_connections();

        tokio::select! {
            () = cancel_token.cancelled() => {
                let _ = self.routing.cleanup();
                stack_to_tun.abort();
                tun_to_stack.abort();
            }
            _ = &mut stack_to_tun => {
                let _ = self.routing.cleanup();
                tun_to_stack.abort();
            }
            _ = &mut tun_to_stack => {
                let _ = self.routing.cleanup();
                stack_to_tun.abort();
            }
            () = tcp_handler => {
                let _ = self.routing.cleanup();
                stack_to_tun.abort();
                tun_to_stack.abort();
            }
        }
    }

    async fn handle_tcp_connections(&mut self) {
        let config = self.config.clone();
        let resolver = self.resolver.clone();

        while let Some((tcp_stream, peer_addr, dest_addr)) = self.tcp_listener.next().await {
            let config = config.clone();
            let resolver = resolver.clone();

            tokio::spawn(async move {
                let dest_ip = dest_addr.ip();
                let dest_port = dest_addr.port();
                let source_ip = peer_addr.ip();
                let source_port = peer_addr.port();

                if let IpAddr::V4(dest_ip) = dest_ip {

                    let target = match resolver.as_ref() {
                        Some(resolver) if resolver.is_fake_ip(dest_ip) => {
                            resolver.get_domain_by_fake_ip(dest_ip)
                                .map_or_else(|| format!("{dest_ip}:{dest_port}"), |host| format!("{host}:{dest_port}"))
                        },
                        _ => format!("{dest_ip}:{dest_port}"),
                    };

                    info!("creating tunnel for {source_ip}:{source_port} -> {target}:{dest_port}");

                    if let Err(error) = Self::socks5_tunnel(&config, &target, tcp_stream).await {
                        error!(%error, "socks5 tunnel error");
                    }
                }
            });
        }
    }

    async fn socks5_tunnel(config: &Config, target: &str, client_stream: NetstackTcpStream) -> Result<(), AppError> {
        let stream = TokioTcpStream::connect(config.server).await.map_err(|_| AppError::TargetUnreachable)?;
        let mut session = Socks5Session::new(config.clone(), stream);

        if session.handshake().await? == consts::s5::auth::AUTH { session.auth().await?; }

        if session.connect(ConnectTarget::Direct(target)).await.is_err() {
            error!("failed socks5 connect to {target}");
            return Ok(());
        }

        let socks5_stream = session.server.take().unwrap();

        let (mut client_read, mut client_write) = tokio::io::split(client_stream);
        let (mut socks5_read, mut socks5_write) = tokio::io::split(socks5_stream);

        let mut client_to_socks5 = tokio::spawn(async move {
            let mut buffer = [0; consts::buffer::PROXY];
            loop {
                match client_read.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Err(error) = socks5_write.write_all(&buffer[..n]).await {
                            error!(%error, "error writing to socks5");
                            break;
                        }
                    }
                    Err(error) => {
                        error!(%error, "error reading from client");
                        break;
                    }
                }
            }
        });

        let mut socks5_to_client = tokio::spawn(async move {
            let mut buffer = [0; consts::buffer::PROXY];
            loop {
                match socks5_read.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Err(error) = client_write.write_all(&buffer[..n]).await {
                            error!(%error, "error writing to client");
                            break;
                        }
                    }
                    Err(error) => {
                        error!(%error, "error reading from socks5");
                        break;
                    }
                }
            }
        });

        tokio::select! {
            _ = &mut client_to_socks5 => socks5_to_client.abort(),
            _ = &mut socks5_to_client => client_to_socks5.abort()
        }

        Ok(())
    }
}