use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use crate::http::{Http, Method};
use crate::prelude::*;

pub struct Socks5Config {
    pub mode: Mode,
    pub server: SocketAddr,

    //Proxy mode
    pub listen: SocketAddr,

    //Cli mode
    pub auth: Option<(String, String)>,
    pub target: Option<Atyp>,
    pub http: Http,
    pub use_tls: bool,
    pub xor: Option<u8>,
}

impl Debug for Socks5Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Socks5Config");
        s.field("mode", &self.mode);
        s.field("server", &self.server);

        match self.mode {
            Mode::Cli => {
                s.field("auth", &self.auth);
                s.field("target", &self.target);
                s.field("http", &self.http);
                s.field("use_tls", &self.use_tls);
                s.field("xor", &self.xor);
            },
            Mode::Proxy | Mode::_Tun => {
                s.field("listen", &self.listen);
            },
        }
        s.finish()
    }
}

impl Default for Socks5Config  {
    fn default() -> Self {
        Self { 
            mode: Mode::Cli, 
            server: SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)),
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 1081)),
            auth: None, 
            target: None, 
            http: Http::default(), 
            use_tls: false,
            xor: None
        }
    }
}

impl Config for Socks5Config {
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

            //Proxy mode
            "--listen" => {
                self.listen = value.parse().map_err(|_| AppError::Arguments("invalid local addr".into()))?;
                Ok(())
            }

            //Cli mode
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
            Mode::Cli => {
                if self.target.is_none() {
                    return Err(AppError::Arguments("missed param --target".into()));
                }
                if self.use_tls && self.target.as_ref().unwrap().host_str().parse::<IpAddr>().is_ok() {
                    return Err(AppError::Arguments("invalid target: https requires domain name, not IP".into()));
                }
            },
            Mode::Proxy => {
                if self.listen.port() == 0 {
                    return Err(AppError::Arguments("port cannot be 0".into()));
                }
            },
            Mode::_Tun => return Err(AppError::Arguments("mode not yet implemented".into())),
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

use super::*;

    #[test]
    fn test_valid_args() {
        let args = vec!["program", "--mode", "cli", "--server", "127.0.0.1:1080", "--target", "https://example.com:8443"];
        let mut config = Socks5Config::from_args(args).unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.mode, Mode::Cli);
        assert_eq!(config.server, SocketAddr::from((Ipv4Addr::LOCALHOST, 1080)));
        assert_eq!(config.use_tls, true);
    }

    #[test]
    fn test_valid_args_with_http_headers() {
        let args = vec![
            "program", 
            "--mode", "cli", 
            "--server", "127.0.0.1:1080", 
            "--target", "https://example.com",
            "--headers", "Content-Type:application/json",
            "--headers", "Authorization:Bearer qwerty123",
        ];
        let mut config = Socks5Config::from_args(args).unwrap();
        assert!(config.validate().is_ok());

        let headers = config.http.headers.unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers.first().unwrap(), &("Content-Type".to_string(), "application/json".to_string()));
        assert_eq!(headers.last().unwrap(), &("Authorization".to_string(), "Bearer qwerty123".to_string()));
    }

    #[test]
    fn test_https_with_ip() {
        let args = vec!["program", "--mode", "cli", "--server", "127.0.0.1:1080", "--target", "https://34.234.10.121/get"];
        let mut config = Socks5Config::from_args(args).unwrap();
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("https requires domain name"));
    }
}