use std::process::Command;

use crate::support::{test_client::TestClient, test_server::TestServer};

mod support;

#[test]
fn test_client_proxy() {
    let port: u16 = 33340;
    
    let _server = TestServer::start(port, None, None);
    let _client = TestClient::start(&format!("127.0.0.1:{port}"), "proxy", None);

    let output = Command::new("curl")
        .arg("-x")
        .arg("socks5://127.0.0.1:1081") //* default client listen addr
        .arg("http://httpbin.org/get")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"url\": \"http://httpbin.org/get\""));
}