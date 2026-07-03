mod support;

use std::io::{Read, Write};

use crate::support::test_server::TestServer;

#[test]
fn test_server_handshake() {
    let server = TestServer::start(33333, None, None);
    let mut client = server.client();

    client.write_all(&[0x05, 0x01, 0x00]).unwrap();
    let mut buf = [0; 2];
    client.read_exact(&mut buf).unwrap();

    assert_eq!(&buf, &[0x05, 0x00]);
}

#[test]
fn test_server_connect() {
    let server = TestServer::start(33334, None, None);
    let mut client = server.client();

    // handshake
    client.write_all(&[0x05, 0x01, 0x00]).unwrap();
    let mut buf = [0; 2];
    client.read_exact(&mut buf).unwrap();

    // connect
    client.write_all(&[
        0x05, 0x01, 0x00, 0x03,
        0x0b, // domain length: 11 bytes
        b'h', b't', b't', b'p', b'b', b'i', b'n', b'.', b'o', b'r', b'g', // httpbin.org
        0x01, 0xbb // port: 443
    ]).unwrap();
    let mut buf = [0; 10];
    client.read_exact(&mut buf).unwrap();

    assert_eq!(buf[0], 0x05);
    assert_eq!(buf[1], 0x00);
}

#[test]
fn test_server_auth() {
    let username = String::from("admin");
    let password = String::from("12345");

    let server = TestServer::start(33335, Some((username.clone(), password.clone())), None);
    let mut client = server.client();

    // handshake
    client.write_all(&[0x05, 0x01, 0x02]).unwrap();
    let mut buf = [0; 2];
    client.read_exact(&mut buf).unwrap();

    assert_eq!(&buf, &[0x05, 0x02]);

    // auth
    let mut response = Vec::with_capacity(1 + 1 + username.len() + 1 + password.len());
    response.push(0x01); // auth version
    response.push(username.len() as u8); // ulen
    response.extend_from_slice(username.as_bytes()); // uname
    response.push(password.len() as u8); // plen
    response.extend_from_slice(password.as_bytes()); // passwd

    client.write_all(&response).unwrap();
    let mut buf = [0; 2];
    client.read_exact(&mut buf).unwrap();

    assert_eq!(&buf, &[0x01, 0x00]);
}