use url::Url;
use tracing::trace;

use crate::AppError;

pub fn collect_args<I, S>(iter: I) -> Result<Vec<(String, String)>, AppError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    iter.into_iter()
        .skip(1)
        .map(|s| s.as_ref().to_string())
        .collect::<Vec<String>>()
        .chunks(2)
        .map(|chunk| {
            let [key, value] = chunk else { 
                return Err(AppError::Arguments(format!("invalid argument format {chunk:?} (expected --key value)"))); 
            };
            if !key.starts_with("--") { return Err(AppError::Arguments(format!("invalid argument syntax: {key} (must start with --)"))); }
            Ok((key.clone(), value.clone()))
        })
        .collect()
}

pub fn parse_url(target: &str) -> Result<(String, u16), AppError> {
    let url = Url::parse(target).map_err(|_| AppError::InvalidDomain)?;

    let host = url.host_str().ok_or(AppError::InvalidDomain)?.to_string();

    let port = match url.port() {
        Some(p) => p,
        None => match url.scheme() {
            "https" => 443,
            "http" => 80,
            _ => return Err(AppError::InvalidDomain),
        },
    };

    Ok((host, port))
}

pub fn extract_path(url: &str) -> String {
    Url::parse(url)
        .map_or("/".to_string(), |url| {
            let mut path = url.path().to_string();
            if path.is_empty() {
                path = "/".to_string();
            }
            if let Some(query) = url.query() {
                path.push('?');
                path.push_str(query);
            }
            path
        })
}

pub fn add_xor(xor: Option<u8>, buf: &mut [u8]) -> &[u8] {
    if let Some(xor) = xor {
        trace!(?buf, "xor");
        for b in buf.iter_mut() { *b ^= xor; }
    }
    buf
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_valid_args() {
        let args = vec!["program", "--key", "value"];
        assert!(collect_args(args).is_ok());
    }

    #[test]
    fn test_invalid_args() {
        let args = vec!["program", "key", "value"];
        assert!(collect_args(args).is_err());
    }

    #[test]
    fn test_missing_value() {
        let args = vec!["program", "--key"];
        assert!(collect_args(args).is_err());
    }

    #[test]
    fn test_parse_url() {
        let (host, port) = parse_url("https://example.com/path").unwrap();
        assert_eq!(host, String::from("example.com"));
        assert_eq!(port, 443);
    }

    #[test]
    fn test_extract_path() {
        assert_eq!(extract_path("https://example.com/api/v1/method-test"), String::from("/api/v1/method-test"));
        assert_eq!(extract_path("127.0.0.1:80"), String::from("/"));
        assert_eq!(extract_path("http://127.0.0.1?key=value"), String::from("/?key=value"));
        assert_eq!(extract_path("http://127.0.0.1/path?key=value"), String::from("/path?key=value"));
    }

    #[test]
    fn test_xor() {
        assert_eq!(add_xor(None, &mut [0x05, 0x01, 0x00]), &[0x05, 0x01, 0x00]);
        assert_eq!(add_xor(Some(0xAA), &mut [0x05, 0x02, 0x00, 0x02]), &[0xAF, 0xA8, 0xAA, 0xA8]);
    }
}