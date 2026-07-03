use crate::prelude::*;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Socks5,
    Tun,
}

impl TryFrom<&str> for Mode {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "s5" => Ok(Self::Socks5),
            "tun" => Ok(Self::Tun),
            _ => Err(AppError::Socks5("invalid mode".into()))
        }
    }
}