use std::{collections::HashMap, net::Ipv4Addr};

use etherparse::{IpNumber, Ipv4Header, PacketBuilder, UdpHeader};
use hickory_proto::{op::{Message, MessageType, UpdateMessage}, rr::{RData, Record, rdata::A}};
use tun::Device;

use crate::{prelude::*, tun::FAKE_IP_POOL};

const FAKE_IP_START: Ipv4Addr = utils::increment_octet(FAKE_IP_POOL);

pub struct FakeDns {
    _fake_to_real: HashMap<Ipv4Addr, Ipv4Addr>,
    domain_to_fake: HashMap<String, Ipv4Addr>,
    next_fake_ip: Ipv4Addr
}

impl FakeDns {
    pub fn new() -> Self {
        Self { _fake_to_real: HashMap::new(), domain_to_fake: HashMap::new(), next_fake_ip: FAKE_IP_START }
    }

    pub fn get_or_create_fake(&mut self, qname: &str) -> Ipv4Addr {
        let domain = qname.trim_end_matches(".");

        //println!("{:?}", domain);

        self.domain_to_fake
            .contains_key(domain)
            .then(|| self.domain_to_fake[domain])
            .unwrap_or_else(|| {
                if !self.domain_to_fake.is_empty() {
                    self.next_fake_ip = utils::increment_octet(self.next_fake_ip);
                }

                self.domain_to_fake.insert(domain.to_string(), self.next_fake_ip);
                self.next_fake_ip
            })
    }
    // TODO https://github.com/JulianSchmid/etherparse add mock for tests

    pub fn build_dns_response(&self, request: &Message, fake_ip: Ipv4Addr) -> Option<Vec<u8>> {
        let mut response = Message::new(request.id(), MessageType::Response, request.op_code);

        if let Some(query) = request.queries.first() { 
            response.add_query(query.clone()); 

            let record = Record::from_rdata(
                query.name().clone(), 
                60, 
                RData::A(A::from(fake_ip))
            );

            response.add_answer(record);

            response.to_vec().ok()
        } else {
            None
        }
    }

    pub fn send_dns_response(
        &self, 
        dev: &Device,
        response: &[u8], 
        src_port: u16, 
        dst_port: u16,
        client_ip: Ipv4Addr,
    ) -> Result<(), AppError> {
        // let ip_header = Ipv4Header::new(
        //     20 + 8 + response.len() as u16, 
        //     64,
        //     IpNumber::UDP, 
        //     fake_ip.octets(), 
        //     client_ip.octets(),
        // ).unwrap();

        // let udp_header = UdpHeader {
        //     source_port: src_port,
        //     destination_port: dst_port,
        //     length: (8 + response.len()) as u16,
        //     checksum: 0, //https://github.com/JulianSchmid/etherparse/blob/master/etherparse/examples/write_ipv4_udp.rs
        // };

        // let mut packet = Vec::new();

        // ip_header.write(&mut packet)?;

        // udp_header.write(&mut packet)?;

        // packet.extend_from_slice(response);

        // dev.send(&packet).map_err(|e| AppError::ModeTun(format!("failed to send packet via TUN: {e}")))?;

            let source_ip = Ipv4Addr::new(127, 0, 0, 53);
            let source_ip = Ipv4Addr::new(10, 0, 0, 9);

        let builder = PacketBuilder::ipv4(
            source_ip.octets(), 
            client_ip.octets(), 
            64
        ).udp(dst_port, src_port);

        let mut packet = Vec::with_capacity(builder.size(response.len()));
        builder.write(&mut packet, response).unwrap();

        dev.send(&packet)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_or_create_fake_ip() {
        let mut fake_dns = FakeDns::new();

        let fake_ip1 = fake_dns.get_or_create_fake("cloudflare-dns.com.");
        let fake_ip2 = fake_dns.get_or_create_fake("example.org.");
        let fake_ip3 = fake_dns.get_or_create_fake("cloudflare-dns.com.");
        let fake_ip4 = fake_dns.get_or_create_fake("mobile.events.data.microsoft.com.");

        assert_eq!(fake_ip1, FAKE_IP_START);
        assert_eq!(fake_ip2, Ipv4Addr::new(100, 64, 0, 2));
        assert_eq!(fake_ip3, FAKE_IP_START);
        assert_eq!(fake_ip4, Ipv4Addr::new(100, 64, 0, 3));
    }
}