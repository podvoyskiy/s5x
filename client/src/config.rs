use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use crate::http::{Http, Method};
use crate::prelude::*;

pub struct Config {
    pub mode: Mode,
    pub server: SocketAddr,

    //Tun mode
    pub address: Ipv4Addr,

    //Socks5 mode
    pub auth: Option<(String, String)>,
    pub target: Option<Atyp>,
    pub http: Http,
    pub use_tls: bool,
    pub xor: Option<u8>,
}

impl Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Config");
        s.field("mode", &self.mode);
        s.field("server", &self.server);

        match self.mode {
            Mode::Socks5 => {
                s.field("auth", &self.auth);
                s.field("target", &self.target);
                s.field("http", &self.http);
                s.field("use_tls", &self.use_tls);
                s.field("xor", &self.xor);
            },
            Mode::Tun => {
                s.field("address", &self.address);
            },
        }
        s.finish()
    }
}

impl Default for Config  {
    fn default() -> Self {
        Self { 
            mode: Mode::Socks5, 
            server: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            address: Ipv4Addr::new(10, 0, 0, 9),
            auth: None, 
            target: None, 
            http: Http::default(), 
            use_tls: false,
            xor: None
        }
    }
}

impl ConfigTrait for Config {
    fn set_param(&mut self, key: &str, value: &str) -> Result<(), AppError> {
        match key {
            "--mode" => {
                self.mode = Mode::try_from(value)?;
                Ok(())
            }
            "--server" => {
                self.server = value.parse().map_err(|_| AppError::Arguments("invalid server addr".into()))?;
                Ok(())
            }

            //Tun mode
            "--address" => {
                self.address = value.parse().map_err(|_| AppError::Arguments("invalid tun IP".into()))?;
                Ok(())
            }

            //Socks5 mode
            "--auth" => {
                value
                    .split_once(':')
                    .map(|(user, pass)| self.auth = Some((user.to_string(), pass.to_string())))
                    .ok_or_else(|| AppError::Arguments(format!("invalid auth format: {value} (expected username:password)")))?;
                Ok(())
            }
            "--target" => {
                self.http.path = utils::extract_path(value);
                self.use_tls = value.starts_with("https://");
                Atyp::from_str(value)
                    .map(|atyp| self.target = Some(atyp))
                    .map_err(|_| AppError::Arguments(format!("invalid target: {value}")))?;
                Ok(())
            }
            "--method" => {
                self.http.method = value.parse::<Method>()?;
                Ok(())
            }
            "--data" => {
                self.http.data = Some(value.to_string());
                if self.http.method == Method::GET { self.http.method = Method::POST; }
                Ok(())
            }
             "--headers" => {
                let header = value
                    .split_once(':')
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .ok_or_else(|| AppError::Arguments(format!("invalid headers format: {value} (expected key:value)")))?;
                if let Some(headers) = &mut self.http.headers {
                    headers.push(header);
                } else {
                    self.http.headers = Some(vec![header]);
                }
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
        match self.mode {
            Mode::Socks5 => {
                if self.target.is_none() {
                    return Err(AppError::Arguments("missed param --target".into()));
                }
                if self.use_tls && self.target.as_ref().unwrap().host_str().parse::<IpAddr>().is_ok() {
                    return Err(AppError::Arguments("invalid target: https requires domain name, not IP".into()));
                }
            }
            Mode::Tun => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

use super::*;

    #[test]
    fn test_valid_args() {
        let args = vec!["program", "--mode", "s5", "--server", "127.0.0.1:1080", "--target", "https://example.com:8443"];
        let mut config = Config::from_args(args).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.mode, Mode::Socks5);
        assert_eq!(config.server, SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)));
        assert_eq!(config.use_tls, true);
    }

    #[test]
    fn test_valid_args_with_http_headers() {
        let args = vec![
            "program", 
            "--mode", "s5", 
            "--server", "127.0.0.1:1080", 
            "--target", "https://example.com",
            "--headers", "Content-Type:application/json",
            "--headers", "Authorization:Bearer qwerty123",
        ];
        let mut config = Config::from_args(args).unwrap();
        assert!(config.validate().is_ok());

        let headers = config.http.headers.unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers.first().unwrap(), &("Content-Type".to_string(), "application/json".to_string()));
        assert_eq!(headers.last().unwrap(), &("Authorization".to_string(), "Bearer qwerty123".to_string()));
    }

    #[test]
    fn test_https_with_ip() {
        let args = vec!["program", "--mode", "s5", "--server", "127.0.0.1:1080", "--target", "https://34.234.10.121/get"];
        let mut config = Config::from_args(args).unwrap();
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("https requires domain name"));
    }
}