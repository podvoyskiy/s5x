use std::{io::Read, net::Ipv4Addr};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use hickory_proto::op::{Message, MessageType};
use tokio_util::sync::CancellationToken;
use tun::{AbstractDevice, Device};
use crate::prelude::*;
use crate::tun::Routing;

pub struct TunSession {
    cancel_token: CancellationToken,
    dev: Device,
    routing: Routing
}

impl TunSession {
    pub fn new(config: &Config, cancel_token: CancellationToken) -> Result<Self, AppError> {
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
        tun_config.platform_config(|config| { config.ensure_root_privileges(true); });

        let dev = tun::create(&tun_config)
            .map_err(|e| AppError::ModeTun(format!("failed to create tun interface: {e}")))?;
        
        let tun_index: u32 = dev.tun_index().map_err(|e| AppError::ModeTun(format!("{e}")))?.try_into()?;

        let routing = Routing::new(config.address, tun_index)?;
        routing.setup()?;
        
        Ok(Self { cancel_token, dev, routing })
    }

    pub fn run(&mut self) {
        let mut buf = [0; 4096];

        loop {
            if self.cancel_token.is_cancelled() {
                let _ = self.routing.cleanup();
                break;
            }

            match self.dev.read(&mut buf) {
                Err(e) => {
                    println!("Read error: {:?}", e);
                    break;
                }
                Ok(size) => {
                    if size == 0 { continue; }

                    match SlicedPacket::from_ip(&buf[..size]) {
                        Err(value) => println!("Err {value:?}"),
                        Ok(value) => {
                            match value.transport {
                                Some(TransportSlice::Udp(udp)) => {

                                    if udp.destination_port() != 53 {
                                        continue;
                                    }
                                    println!("{:?}", udp.destination_port());

                                    if let Ok(dns_msg) = Message::from_vec(udp.payload()) {
                                        if dns_msg.message_type == MessageType::Query {
                                            if let Some(_query) = dns_msg.queries.first() {

                                                // let fake_ip = fake_dns.get_or_create_fake(&query.name.to_string());
                                                // //println!("fake ip {fake_ip:?}");

                                                // if let Some(response) = fake_dns.build_dns_response(&dns_msg, fake_ip) {

                                                //     if let Some(NetSlice::Ipv4(ipv4)) = value.net {
                                                //         let res = fake_dns.send_dns_response(
                                                //             &self.dev,
                                                //             &response, 
                                                //             udp.source_port(),
                                                //             udp.destination_port(), 
                                                //             ipv4.header().source_addr()
                                                //         ).unwrap();

                                                //         println!("{:?}", "after");
                                                //     }
                                                // }
                                            }
                                        }
                                    }
                                }
                                Some(TransportSlice::Icmpv4(_icmpv4)) => {
                                    //println!("{:?}", icmpv4); //ping 100.64.0.2
                                }
                                // В блоке match для TCP
                                Some(TransportSlice::Tcp(tcp)) => {
                                    if let Some(NetSlice::Ipv4(_ipv4)) = value.net {
                                        // Если это TCP на порт 53
                                        if tcp.destination_port() == 53 {
                                            println!("{:?}", "tcp 53");
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                },
            }
        }
    }
}