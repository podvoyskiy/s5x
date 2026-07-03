use std::env::args;

use crate::{AppError, utils};

pub trait ConfigTrait: Default + Sized {
    fn new() -> Result<Self, AppError> {
        Self::from_args(args())
    }

    fn from_args<I, S>(iter: I) -> Result<Self, AppError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut config = Self::default();
        for (key, value) in utils::collect_args(iter)? {
            config.set_param(&key, &value)?;
        }
        Ok(config)
    }

    fn parse_byte(&mut self, value: &str) -> Result<u8, AppError> {
        value.strip_prefix("0x")
            .map_or_else(|| value.parse::<u8>(), |hex| u8::from_str_radix(hex, 16))
            .map_err(|_| AppError::Arguments(format!("invalid xor: {value} (expected 0-255 decimal or 0x00-0xFF hex)")))
    }

    fn set_param(&mut self, key: &str, value: &str) -> Result<(), AppError>;
    fn validate(&mut self) -> Result<(), AppError>;
}