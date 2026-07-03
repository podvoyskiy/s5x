use std::net::IpAddr;

use tokio::net::TcpStream;

use crate::prelude::*;

pub fn addr_to_bytes(addr: &TcpStream) -> Result<Vec<u8>, AppError> {
    let addr = addr.local_addr()?;

    let mut ip_as_bytes = match addr.ip() {
        IpAddr::V4(ipv4) => ipv4.octets().to_vec(),
        IpAddr::V6(ipv6) => ipv6.octets().to_vec(),
    };
    let port_as_bytes: [u8; 2] = addr.port().to_be_bytes();
    ip_as_bytes.extend_from_slice(&port_as_bytes);

    Ok(ip_as_bytes)
}

pub fn bytes_to_credentials(buf: &[u8]) -> Result<(String, String), AppError> {
    let mut bytes = buf.iter();

    let _ver = bytes.next().ok_or(AppError::AuthFailed)?;
    let ulen = *bytes.next().ok_or(AppError::AuthFailed)? as usize;
    let user: Vec<u8> = bytes.by_ref().take(ulen).copied().collect();
    let plen = *bytes.next().ok_or(AppError::AuthFailed)? as usize;
    let pass: Vec<u8> = bytes.take(plen).copied().collect();

    let user = String::from_utf8(user).map_err(|_| AppError::AuthFailed)?;
    let pass = String::from_utf8(pass).map_err(|_| AppError::AuthFailed)?;

    debug!(username = ?user, password = ?pass, "auth");

    Ok((user, pass))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bytes_to_credentials() {
        let buf = &[
            0x01,
            0x04,
            b'u', b's', b'e', b'r',
            0x06,
            b'p', b'a', b's', b's', b'w', b'd'
        ];
        let credentials = bytes_to_credentials(buf);
        assert!(credentials.is_ok());
        let (user, pass) = credentials.unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "passwd");
    }
}