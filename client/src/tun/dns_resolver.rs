use std::{collections::HashMap, net::Ipv4Addr};
use hickory_proto::{op::{Message, MessageType}, rr::{RData, Record, rdata::A}};

use crate::prelude::*;
use crate::tun::FAKE_IP_POOL;

const FAKE_IP_START: Ipv4Addr = utils::increment_octet(FAKE_IP_POOL);

pub struct DnsResolver {
    domain_to_fake: HashMap<String, Ipv4Addr>,
    next_fake_ip: Ipv4Addr
}

impl DnsResolver {
    pub fn new() -> Self {
        Self {
            domain_to_fake: HashMap::new(),
            next_fake_ip: FAKE_IP_START,
        }
    }

    pub fn get_or_create_fake(&mut self, qname: &str) -> Ipv4Addr {
        let domain = qname.trim_end_matches('.');

        if let Some(ip) = self.domain_to_fake.get(domain) {
            *ip
        } else {
            if !self.domain_to_fake.is_empty() {
                self.next_fake_ip = utils::increment_octet(self.next_fake_ip);
            }

            self.domain_to_fake.insert(domain.to_string(), self.next_fake_ip);
            self.next_fake_ip
        }
    }

    pub fn build_dns_response(request_data: &[u8], fake_ip: Ipv4Addr) -> Option<Vec<u8>> { //TODO add tests
        if let Ok(request) = Message::from_vec(request_data) {
            let mut response = Message::new(
                request.id, 
                MessageType::Response, 
                request.op_code
            );
            response.metadata.authoritative = true;
            response.metadata.recursion_desired = request.metadata.recursion_desired;
            response.metadata.recursion_available = true;
            response.metadata.truncation = false;
            response.metadata.response_code = hickory_proto::op::ResponseCode::NoError;
            
            for query in request.queries {
                response.add_query(query.clone());
                
                let record = Record::from_rdata(
                    query.name().clone(),
                    60,
                    RData::A(A::from(fake_ip))
                );
                response.add_answer(record);
            }
            
            if let Ok(bytes) = response.to_vec() {
                return Some(bytes);
            }
        }
        
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_or_create_fake_ip() {
        let mut fake_dns = DnsResolver::new();

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