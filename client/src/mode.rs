use crate::prelude::*;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Cli,
    Proxy,
    _Tun,
}

impl TryFrom<&str> for Mode {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "cli" => Ok(Self::Cli),
            "proxy" => Ok(Self::Proxy),
            "tun" => Err(AppError::Socks5(format!("mode {value} not yet implemented"))),
            _ => Err(AppError::Socks5("invalid mode".into()))
        }
    }
}