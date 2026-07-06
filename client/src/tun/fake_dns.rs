use std::{collections::HashMap, net::Ipv4Addr};

use hickory_proto::{op::{Message, MessageType}, rr::{RData, Record, rdata::A}};
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::{prelude::*, tun::FAKE_IP_POOL};

const FAKE_IP_START: Ipv4Addr = utils::increment_octet(FAKE_IP_POOL);

pub struct FakeDns {
    cancel_token: CancellationToken,
    _fake_to_real: HashMap<Ipv4Addr, Ipv4Addr>,
    domain_to_fake: HashMap<String, Ipv4Addr>,
    next_fake_ip: Ipv4Addr
}

impl FakeDns {
    pub fn new(cancel_token: CancellationToken) -> Self {
        Self { cancel_token, _fake_to_real: HashMap::new(), domain_to_fake: HashMap::new(), next_fake_ip: FAKE_IP_START }
    }

    pub fn build_dns_response(&self, request_data: &[u8], fake_ip: Ipv4Addr) -> Option<Vec<u8>> {
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
            
            // Копируем вопросы
            for query in request.queries {
                response.add_query(query.clone());
                
                // Создаем A запись с фейковым IP
                let record = Record::from_rdata(
                    query.name().clone(),
                    60, // TTL 60 секунд
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

    pub async fn run(&mut self) {
        let socket = match UdpSocket::bind("10.0.0.9:53").await {
            Ok(s) => {
                println!("✅ UDP сокет создан на 10.0.0.9:53");
                s
            }
            Err(e) => {
                eprintln!("❌ Ошибка создания UDP сокета: {}", e);
                return;
            }
        };

        let mut buf = vec![0u8; 65536];

         loop {
            match socket.recv_from(&mut buf).await {
                Ok((n, src_addr)) => {
                    let data = &buf[..n];
                    
                    if let Ok(request) = Message::from_vec(data) {
                        if let Some(query) = request.queries.first() {
                            let qname = query.name().to_ascii();
                            let qtype = query.query_type();
                            let qtype_str = format!("{:?}", qtype);
                            
                            let fake_ip = self.get_or_create_fake(&qname);
                            
                            println!(
                                "{} -> {}: {} {} => {}",
                                src_addr,
                                "10.0.0.9:53",
                                qname,
                                qtype_str,
                                fake_ip
                            );
                            
                            if let Some(response) = self.build_dns_response(data, fake_ip) {
                                if let Err(e) = socket.send_to(&response, src_addr).await {
                                    eprintln!("❌ Ошибка отправки ответа: {}", e);
                                } else {
                                    println!("   ↳ Ответ: {} A {}", qname, fake_ip);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Ошибка UDP: {}", e);
                    break;
                }
            }
        }

        loop {
            let _ = socket.recv_from(&mut buf).await;

            println!("{:?}", buf);

            if self.cancel_token.is_cancelled() {
                break;
            }
        }
    
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_or_create_fake_ip() {
        let mut fake_dns = FakeDns::new(CancellationToken::new());

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