use crate::prelude::*;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Socks5,
    Tun2Socks,
    Tun,
}

impl TryFrom<&str> for Mode {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "socks5" | "s5" => Ok(Self::Socks5),
            "tun2socks" | "s5t" => Ok(Self::Tun2Socks),
            "tun" | "t" => Ok(Self::Tun),
            _ => Err(AppError::Other("invalid mode".into()))
        }
    }
}