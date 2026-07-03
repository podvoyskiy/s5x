#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]

pub mod consts;
pub mod colorize;
pub mod errors;
pub mod utils;
pub mod atyp;
pub mod config;

pub use errors::AppError;
pub use atyp::Atyp;
pub use config::Config;