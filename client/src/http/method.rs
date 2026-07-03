#![allow(clippy::upper_case_acronyms)]

use std::{fmt::Display, str::FromStr};
use crate::prelude::*;

#[derive(Debug, PartialEq)]
pub enum Method {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE
}

impl FromStr for Method {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Self::GET),
            "POST" => Ok(Self::POST),
            "PUT" => Ok(Self::PUT),
            "PATCH" => Ok(Self::PATCH),
            "DELETE" => Ok(Self::DELETE),
            _ => Err(AppError::InvalidHttpMethod)
        }
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::PATCH => "PATCH",
            Method::DELETE => "DELETE",
        };
        write!(f, "{str}")
    }
}