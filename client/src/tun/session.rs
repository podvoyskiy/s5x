use std::{io::Read, net::Ipv4Addr};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
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

            if let Ok(size) = self.dev.read(&mut buf) {
                match SlicedPacket::from_ip(&buf[..size]) {
                    Err(value) => println!("Err {value:?}"),
                    Ok(value) => {
                        match value.transport {
                            Some(TransportSlice::Tcp(_tcp)) => {
                                if let Some(NetSlice::Ipv4(ipv4)) = value.net {
                                    println!("{:?}", ipv4.header().destination_addr());
                                }
                            }
                            Some(TransportSlice::Udp(_udp)) => {}
                            Some(TransportSlice::Icmpv4(_icmpv4)) => {}
                            Some(TransportSlice::Icmpv6(_icmpv6)) => {}
                            None => {},
                        }
                    }
                }
            } else {
                let _ = self.routing.cleanup();
                break;
            }
        }
    }
}