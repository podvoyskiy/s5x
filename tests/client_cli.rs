use crate::support::{test_client::TestClient, test_server::TestServer};

mod support;

#[test]
fn test_client() {
    let port: u16 = 33336;
    let _server = TestServer::start(port, None, None);
    let _client = TestClient::start(&format!("127.0.0.1:{port}"), "cli", Some("http://httpbin.org/get"));
}

#[test]
fn test_http() {
    let port: u16 = 33337;
    let _server = TestServer::start(port, None, None);
    let output = TestClient::run(
        &format!("127.0.0.1:{port}"), 
        "http://httpbin.org/post", 
        None,
        Some("{\"key\":\"value\"}"), 
        None,
        None
    );

    assert!(output.contains("200 OK"));
    assert!(output.contains("\"url\": \"http://httpbin.org/post\""));
    assert!(output.contains("\"key\": \"value\""));
}

#[test]
fn test_https() {
    let port: u16 = 33338;
    let _server = TestServer::start(port, None, None);
    let output = TestClient::run(
        &format!("127.0.0.1:{port}"), 
        "https://httpbin.org/post", 
        None,
        Some("{\"key\":\"value\"}"),
        Some("User-Agent:curl/8.5.0"),
        None
    );

    assert!(output.contains("200 OK"));
    assert!(output.contains("\"url\": \"https://httpbin.org/post\""));
    assert!(output.contains("\"key\": \"value\""));
    assert!(output.contains("\"User-Agent\": \"curl/8.5.0\""));
}

#[test]
fn test_xor() {
    let port: u16 = 33339;
    let username = String::from("admin");
    let password = String::from("12345");
    let xor = 0xAA;

    let _server = TestServer::start(port, Some((username.clone(), password.clone())), Some(xor));
    let output = TestClient::run(
        &format!("127.0.0.1:{port}"), 
        "https://httpbin.org/get", 
        Some((username.clone(), password.clone())),
        None,
        None,
        Some(xor)
    );

    assert!(output.contains("200 OK"));
    assert!(output.contains("\"url\": \"https://httpbin.org/get\""));
}