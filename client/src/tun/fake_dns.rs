use std::{collections::HashMap, net::Ipv4Addr};

use hickory_proto::op::Message;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;
use crate::tun::DnsResolver;

pub struct FakeDns {
    resolver: DnsResolver,
    cancel_token: CancellationToken,
    udp_socket: UdpSocket,
    _fake_to_real: HashMap<Ipv4Addr, Ipv4Addr>,
}

impl FakeDns {
    pub async fn new(cancel_token: CancellationToken) -> Result<Self, AppError> {
        match UdpSocket::bind("10.0.0.9:53").await {
            Ok(udp_socket) => {
                Ok(Self { resolver: DnsResolver::new(), udp_socket, cancel_token, _fake_to_real: HashMap::new() })
            }
            Err(_) => Err(AppError::ModeTun("failed to create udp socket".into()))
        }
    }

    pub async fn run(&mut self) {
        let mut buf = vec![0u8; 65536];

         loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    break;
                }

                result = self.udp_socket.recv_from(&mut buf) => {
                    match result {
                        Ok((n, src_addr)) => {
                            let data = &buf[..n];
                    
                            if let Ok(request) = Message::from_vec(data) {
                                if let Some(query) = request.queries.first() {
                                    let qname = query.name().to_ascii();
                                    let qtype = query.query_type();
                                    let qtype_str = format!("{:?}", qtype);
                                    
                                    let fake_ip = self.resolver.get_or_create_fake(&qname);
                                    
                                    println!(
                                        "{} -> {}: {} {} => {}",
                                        src_addr,
                                        "10.0.0.9:53",
                                        qname,
                                        qtype_str,
                                        fake_ip
                                    );
                                    
                                    if let Some(response) = DnsResolver::build_dns_response(data, fake_ip) {
                                        if let Err(e) = self.udp_socket.send_to(&response, src_addr).await {
                                            eprintln!("error response udp : {e}");
                                        } else {
                                            println!("  Response: {} A {}", qname, fake_ip);
                                        }
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("UDP error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
}