use std::net::Ipv4Addr;
use std::str::FromStr;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Socks5Config {
    pub host: Ipv4Addr,
    pub port: u16,
    pub auth: Option<(String, String)>,
    pub xor: Option<u8>,
}

impl Default for Socks5Config  {
    fn default() -> Self {
        Self { host: Ipv4Addr::LOCALHOST, port: 1080, auth: None, xor: None }
    }
}

impl Config for Socks5Config {
    fn set_param(&mut self, key: &str, value: &str) -> Result<(), AppError> {
        match key {
            "--host" => {
                Ipv4Addr::from_str(value)
                    .map(|host| self.host = host)
                    .map_err(|_| AppError::Arguments(format!("invalid host: {value}")))?;
                Ok(())
            }
            "--port" => {
                value.parse()
                    .map(|port| self.port = port)
                    .map_err(|_| AppError::Arguments(format!("invalid port: {value}")))?;
                Ok(())
            }
            "--auth" => {
                value
                    .split_once(':')
                    .map(|(user, pass)| self.auth = Some((user.to_string(), pass.to_string())))
                    .ok_or_else(|| AppError::Arguments(format!("invalid auth format: {value} (expected username:password)")))?;
                Ok(())
            }
            "--xor" => {
                self.xor = Some(self.parse_byte(value)?);
                Ok(())
            }
            _ => Err(AppError::Arguments(format!("unknown argument {key}")))
        }
    }

    fn validate(&mut self) -> Result<(), AppError> {
        if self.port == 0 { return Err(AppError::Arguments("port cannot be 0".into())); }
        
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_valid_args() {
        let args = vec!["program", "--host", "127.0.0.1", "--port", "3000"];
        let config = Socks5Config::from_args(args).unwrap();
        assert_eq!(config.host, (Ipv4Addr::LOCALHOST));
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_invalid_host() {
        let args = vec!["program", "--host", "256.256.256.256"];
        assert!(Socks5Config::from_args(args).is_err());
    }

    #[test]
    fn test_invalid_port() {
        let args = vec!["program", "--port", "foo"];
        assert!(Socks5Config::from_args(args).is_err());
    }

    #[test]
    fn test_auth() {
        let args = vec!["program", "--auth", "user:pass"];
        assert!(Socks5Config::from_args(args).is_ok());
    }
}