use tokio::io::AsyncReadExt;
use std::fmt::Write;

use crate::prelude::*;
use crate::http::Method;

#[derive(Debug)]
pub struct Http {
    pub method: Method,
    pub path: String,
    pub data: Option<String>,
    pub headers: Option<Vec<(String, String)>>
}

impl Http {
    pub fn default() -> Self {
        Self { method: Method::GET, path: "/".to_string(), data: None, headers: None }
    }

    pub fn build_request(&self, host: &str) -> String {
        let mut request = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", 
            self.method, 
            self.path,
            host
        );

        if !self.headers.as_ref().is_some_and(|h| h.iter().any(|(k, _)| k == "User-Agent")) {
            write!(request, "User-Agent: {}/{}\r\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).unwrap();
        }

        if let Some(headers) = &self.headers {
            for (k, v) in headers {
                write!(request, "{k}: {v}\r\n").unwrap();
            }
        }

        if let Some(data) = &self.data {
            write!(request, "Content-Length: {}\r\n", data.len()).unwrap();
            request.push_str("\r\n");
            request.push_str(data);
        } else {
            request.push_str("\r\n");
        }
        request
    }

    pub async fn read_response(stream: &mut (impl AsyncReadExt + Unpin)) -> Result<Vec<u8>, AppError> {
        let mut buf = Vec::new();
        let mut headers_found = false;
        let mut content_length: Option<usize> = None;

        loop {
            let mut chunk: Vec<u8> = vec![0; 1024];
            let n = stream.read(&mut chunk).await?;
            if n == 0 { break; }

            buf.extend_from_slice(&chunk[..n]);

            if !headers_found && let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                headers_found = true;
                let headers = &buf[..pos];

                if let Some(len) = Self::extract_content_length(headers) {
                    content_length = Some(len);
                }
            }

            if let Some(len) = content_length {
                let header_end = buf.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
                let body_len = buf.len() - (header_end + 4);
                
                if body_len >= len { break; }
            }
        }

        Ok(buf)
    }

    pub fn print_response(buf: &[u8]) -> Result<(), AppError> {
        let response = String::from_utf8_lossy(buf);
        match response.find("\r\n\r\n") {
            Some(pos) => {
                debug!("\n---headers---\n{}\n---headers---\n", &response[..pos]);
                let body = &response[pos + 4..];
                println!("{body}");
                Ok(())
            },
            None => Err(AppError::InvalidHttpResponse),
        }
    }

    fn extract_content_length(headers: &[u8]) -> Option<usize> {
        let headers_str = String::from_utf8_lossy(headers);
        for line in headers_str.lines() {
            if let Some(len) = line.strip_prefix("Content-Length:") {
                return len.trim().parse().ok();
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor; 

    #[test]
    fn test_build_request() {
        let http = Http::default();
        let request = http.build_request("example.com");

        assert!(request.starts_with("GET / HTTP/1.1\r\n"));
        assert!(request.contains("Host: example.com\r\n"));
        assert!(request.contains(&format!("User-Agent: {}/{}\r\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))));
        assert!(request.ends_with("\r\n"));
        assert!(!request.contains("Content-Length:"));
    }

    #[test]
    fn test_build_request_with_headers_and_data() {
        let http = Http {
            method: Method::POST,
            path: "/api/data".to_string(),
            data: Some("{\"key\":\"value\"}".to_string()),
            headers: Some(vec![("Content-Type".to_string(), "application/json".to_string())]),
        };
        let request = http.build_request("api.example.com");
        
        assert!(request.contains("Content-Type: application/json\r\n"));
        assert!(request.contains("Content-Length: 15\r\n"));
        assert!(request.contains("\r\n\r\n{\"key\":\"value\"}"));
    }

    #[test]
    fn test_extract_content_length() {
        let headers = b"HTTP/1.1 200 OK\r\nContent-Length: 123\r\nContent-Type: text/html\r\n";
        let result = Http::extract_content_length(headers);
        assert_eq!(result, Some(123));
    }

    #[test]
    fn test_print_response() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<body>Hello</body>";
        let result = Http::print_response(response);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_read_response() {
        let mock_response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nHello";
        let mut mock_stream = Cursor::new(mock_response);
        
        let result = Http::read_response(&mut mock_stream).await.unwrap();
        assert_eq!(result, mock_response);
    }
}