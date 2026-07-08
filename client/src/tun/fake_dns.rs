use std::net::SocketAddr;
use std::{collections::HashMap, net::Ipv4Addr};

use hickory_proto::op::Message;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;
use crate::tun::DnsResolver;

const MAX_DNS_UDP_PACKET_SIZE: usize = 65536;

pub struct FakeDns {
    cancel_token: CancellationToken,
    resolver: DnsResolver,
    udp_socket: UdpSocket,
    udp_socket_addr: SocketAddr,
    _fake_to_real: HashMap<Ipv4Addr, Ipv4Addr>,
}

impl FakeDns {
    pub async fn new(config: &Config, resolver: DnsResolver, cancel_token: CancellationToken) -> Result<Self, AppError> {
        match UdpSocket::bind(SocketAddr::from((config.address, 53))).await {
            Ok(udp_socket) => {
                let udp_socket_addr = udp_socket.local_addr()?;
                Ok(Self { cancel_token, resolver, udp_socket, udp_socket_addr, _fake_to_real: HashMap::new() })
            }
            Err(_) => Err(AppError::ModeTun("failed to create udp socket".into()))
        }
    }

    pub async fn run(&mut self) {
        let mut buf = vec![0u8; MAX_DNS_UDP_PACKET_SIZE];

         loop {
            tokio::select! {
                () = self.cancel_token.cancelled() => {
                    break;
                }

                result = self.udp_socket.recv_from(&mut buf) => {
                    match result {
                        Ok((n, src_addr)) => {
                            let data = &buf[..n];
                    
                            if let Ok(request) = Message::from_vec(data) && let Some(query) = request.queries.first() {
                                let qname = query.name().to_ascii();
                                let qtype = query.query_type();
                                
                                let fake_ip = self.resolver.get_or_create_fake_ip(&qname);

                                trace!("{src_addr} -> {}: {qname} {qtype} => {fake_ip}", self.udp_socket_addr);

                                if let Some(response) = DnsResolver::build_dns_response(data, fake_ip)
                                    && let Err(error) = self.udp_socket.send_to(&response, src_addr).await {
                                    error!(%error, "failed to send data on the udp socket");
                                }
                            }
                        },
                        Err(error) => {
                            error!(%error, "failed to receive message on the udp socket");
                            break;
                        }
                    }
                }
            }
        }
    }
}