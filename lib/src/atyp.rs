use std::{net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs}, str::FromStr};

use tracing::debug;

use crate::{AppError, consts, utils};

#[derive(Debug, PartialEq)]
pub enum Atyp {
    Domain((String, u16)),
    Ipv4(SocketAddrV4),
    Ipv6(SocketAddrV6),
}

impl Atyp {
    pub fn as_u8(&self) -> u8 {
        match self {
            Atyp::Domain(_) => consts::connect::ATYP_DOMAINNAME,
            Atyp::Ipv4(_) => consts::connect::ATYP_IPV4,
            Atyp::Ipv6(_) => consts::connect::ATYP_IPV6,
        }
    }

    pub fn host_str(&self) -> String {
        match self {
            Atyp::Domain((host, _)) => host.clone(),
            Atyp::Ipv4(addr) => addr.ip().to_string(),
            Atyp::Ipv6(addr) => addr.ip().to_string(),
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            Atyp::Domain((_, port)) => *port,
            Atyp::Ipv4(addr) => addr.port(),
            Atyp::Ipv6(addr) => addr.port(),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Atyp::Domain((host, port)) => {
                let mut bytes: Vec<u8> = Vec::with_capacity(1 + 1 + host.len() + 2);
                bytes.push(self.as_u8());
                bytes.push(host.len() as u8);
                bytes.extend_from_slice(host.as_bytes());
                bytes.extend(port.to_be_bytes());
                bytes
            },
            Atyp::Ipv4(socket_addr) => {
                let mut bytes: Vec<u8> = Vec::with_capacity(1 + 4 + 2);
                bytes.push(self.as_u8());
                bytes.extend(socket_addr.ip().to_bits().to_be_bytes());
                bytes.extend(socket_addr.port().to_be_bytes());
                bytes
            },
            Atyp::Ipv6(socket_addr) => {
                let mut bytes: Vec<u8> = Vec::with_capacity(1 + 16 + 2);
                bytes.push(self.as_u8());
                bytes.extend(socket_addr.ip().to_bits().to_be_bytes());
                bytes.extend(socket_addr.port().to_be_bytes());
                bytes
            },
        }
    }

    pub fn to_socket_addr(&self) -> Result<SocketAddr, AppError> {
        match self {
            Atyp::Domain((host, port)) => {
                let addrs = (host.as_str(), *port).to_socket_addrs().map_err(|_| AppError::InvalidDomain)?;
                let addrs: Vec<SocketAddr> = addrs.collect();
                
                // prefer IPv4. fallback to IPv6.
                if let Some(addr) = addrs.iter().find(|addr| addr.is_ipv4()) {
                    Ok(*addr)
                }
                else if let Some(addr) = addrs.iter().find(|addr| addr.is_ipv6()) {
                    Ok(*addr)
                }
                else {
                    Err(AppError::TargetUnreachable)
                }
            },
            Atyp::Ipv4(socket_addr_v4) => Ok(SocketAddr::V4(*socket_addr_v4)),
            Atyp::Ipv6(socket_addr_v6) => Ok(SocketAddr::V6(*socket_addr_v6)),
        }
    }

    pub fn from_bytes(mut buf: Vec<u8>) ->Result<Self, AppError> {
        let atyp = buf.remove(0);

        match atyp {
            consts::connect::ATYP_DOMAINNAME => {
                // 1 byte is domain length, followed by the domain, then 2 bytes for the port
                let domain_len = *buf.first().ok_or(AppError::InvalidDomain)? as usize;
                let domain_bytes = buf.get(1..1 + domain_len).ok_or(AppError::InvalidDomain)?;
                let port_bytes = buf.get(1 + domain_len..1 + domain_len + 2).ok_or(AppError::InvalidDomain)?;

                let domain = String::from_utf8_lossy(domain_bytes);
                let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);

                debug!(%domain, port, "resolving domain name");

                Ok(Self::Domain((domain.to_string(), port)))
            },
            consts::connect::ATYP_IPV4 => {
                if buf.len() != 6 { return Err(AppError::InvalidIpv4); }

                let ip = Ipv4Addr::new(buf[0], buf[1], buf[2], buf[3]);
                let port = u16::from_be_bytes([buf[4], buf[5]]);
                Ok(Self::Ipv4(SocketAddrV4::new(ip, port)))
            },
            consts::connect::ATYP_IPV6 => {
                if buf.len() != 18 { return Err(AppError::InvalidIpv6); }

                let ip_bytes: [u8; 16] = buf[0..16].try_into().map_err(|_| AppError::InvalidIpv6)?;
                let port = u16::from_be_bytes([buf[16], buf[17]]);
                Ok(Self::Ipv6(SocketAddrV6::new(Ipv6Addr::from(ip_bytes), port, 0, 0)))
            },
            _ => Err(AppError::InvalidAtyp)
        }
    }
}

impl FromStr for Atyp {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse::<SocketAddr>() {
            return Ok(match value {
                SocketAddr::V4(socket_addr_v4) => Atyp::Ipv4(socket_addr_v4),
                SocketAddr::V6(socket_addr_v6) => Atyp::Ipv6(socket_addr_v6),
            });
        }

        match utils::parse_url(s) {
            Ok((host, port)) => Ok(Atyp::Domain((host, port))),
            Err(_) => Err(AppError::InvalidAtyp),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_str() {
        assert!(Atyp::from_str("127.0.0.1:80").is_ok());
        assert!(Atyp::from_str("[2001:4860:4860::8888]:53").is_ok());
        assert!(Atyp::from_str("https://example.com").is_ok());
        assert!(Atyp::from_str("https://api.example.com/some/path").is_ok());
        assert!(Atyp::from_str("http://sub.domain.com:8080").is_ok());
        assert!(Atyp::from_str("https://").is_err());
        assert!(Atyp::from_str("invalid host").is_err());
    }

    #[test]
    fn test_from_bytes_ipv4() {
        // IPv4: 8.8.8.8:53 (Google DNS)
        let buf = &[consts::connect::ATYP_IPV4, 0x08, 0x08, 0x08, 0x08, 0x00, 0x35];
        assert!(Atyp::from_bytes(buf.to_vec()).is_ok());
    }

    #[test]
    fn test_from_bytes_ipv6() {
        // IPv6: 2001:4860:4860::8888:53 (Google DNS)
        let buf = &[
            consts::connect::ATYP_IPV6,
            0x20, 0x01, 0x48, 0x60, 0x48, 0x60, 0x00, 0x00, 
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x88, 
            0x00, 0x35
        ];
        assert!(Atyp::from_bytes(buf.to_vec()).is_ok());
    }

    #[test]
    fn test_from_bytes_domain_name() {
        let buf = &[
            consts::connect::ATYP_DOMAINNAME,
            0x0a, // domain length: 10 bytes
            b'g', b'o', b'o', b'g', b'l', b'e', b'.', b'c', b'o', b'm', // google.com
            0x01, 0xbb // port: 443
        ];
        assert!(Atyp::from_bytes(buf.to_vec()).is_ok());
    }
}