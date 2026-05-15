use std::str::FromStr;

use serde::{Deserialize, Serialize};
use crate::parser_error;
use crate::language::Error;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Eq, Hash)]
pub enum RDLTypes {
    String(String),
    Number(i64),
    Vector(Vec<RDLTypes>),
    Boolean(bool),
}

impl From<RDLTypes> for u16 {
    fn from(value: RDLTypes) -> Self {
        match value {
            RDLTypes::Boolean(b) => if b { 1 } else { 0 },
            RDLTypes::Number(n) => n as u16,
            RDLTypes::String(s) => s.parse::<u16>().unwrap_or(0),
            RDLTypes::Vector(v) => v.into_iter().map(u16::from).sum(),
        }
    }
}

impl TryFrom<RDLTypes> for String {
    type Error = Error;

    fn try_from(value: RDLTypes) -> Result<Self, Error> {
        if let RDLTypes::String(s) = value {
            Ok(s)
        } else {
            parser_error!(format!("Error with conversion! IN TryFrom<RDLTypes> for String\n{value}\n"), line!() as usize, column!() as usize)
        }
    }
}

impl TryFrom<RDLTypes> for i64 {
    type Error = Error;

    fn try_from(value: RDLTypes) -> Result<Self, Error> {
        if let RDLTypes::Number(n) = value {
            Ok(n)
        } else {
            parser_error!("Error with conversion! IN TryFrom<RDLTypes> for i64", line!() as usize, column!() as usize)
        }
    }
}

impl TryFrom<RDLTypes> for Vec<RDLTypes> {
    type Error = Error;

    fn try_from(value: RDLTypes) -> Result<Self, Error> {
        if let RDLTypes::Vector(v) = value {
            Ok(v)
        } else {
            parser_error!("Error with conversion! IN TryFrom<RDLTypes> for Vec<RDLTypes>", line!() as usize, column!() as usize)
        }
    }
}

impl TryFrom<RDLTypes> for bool {
    type Error = Error;

    fn try_from(value: RDLTypes) -> Result<Self, Error> {
        if let RDLTypes::Boolean(b) = value {
            Ok(b)
        } else {
            parser_error!("Error with conversion! IN TryFrom<RDLTypes> for bool", line!() as usize, column!() as usize)
        }
    }
}

// impl TryFrom<RDLTypes> for f64 {
//     type Error = Error;
//
//     fn try_from(value: RDLTypes) -> Result<Self, Error> {
//         if let RDLTypes::Number(n) = value {
//             Ok(n as f64)
//         } else {
//             parser_error!("Error with conversion!", line!() as usize, column!() as usize)
//         }
//     }
// }

impl TryFrom<RDLTypes> for i128 {
    type Error = Error;

    fn try_from(value: RDLTypes) -> Result<Self, Error> {
        if let RDLTypes::Number(n) = value {
            Ok(n as i128)
        } else {
            parser_error!("Error with conversion! IN TryFrom<RDLTypes> for i128", line!() as usize, column!() as usize)
        }
    }
}

impl TryFrom<RDLTypes> for usize {
    type Error = Error;

    fn try_from(value: RDLTypes) -> Result<Self, Error> {
        if let RDLTypes::Number(n) = value {
            Ok(n as usize)
        } else {
            parser_error!("Error with conversion! IN TryFrom<RDLTypes> for usize", line!() as usize, column!() as usize)
        }
    }
}

impl From<usize> for RDLTypes {
    fn from(value: usize) -> Self {
        RDLTypes::Number(value as i64)
    }
}

impl From<u16> for RDLTypes {
    fn from(value: u16) -> Self {
        RDLTypes::Number(value as i64)
    }
}

impl From<&RDLTypes> for u16 {
    fn from(value: &RDLTypes) -> Self {
        match value {
            RDLTypes::Boolean(b) => if *b { 1 } else { 0 },
            RDLTypes::Number(n) => *n as u16,
            RDLTypes::String(s) => s.parse::<u16>().unwrap_or(0),
            RDLTypes::Vector(v) => v.iter().map(u16::from).sum(), 
        }
    }
}

impl From<String> for RDLTypes {
    fn from(value: String) -> Self {
        RDLTypes::String(value)
    }
}

impl From<&str> for RDLTypes {
    fn from(value: &str) -> Self {
        RDLTypes::String(value.to_string())
    }
}

impl From<i64> for RDLTypes {
    fn from(value: i64) -> Self {
        RDLTypes::Number(value)
    }
}

impl From<bool> for RDLTypes {
    fn from(value: bool) -> Self {
        RDLTypes::Boolean(value)
    }
}

impl FromStr for RDLTypes {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RDLTypes::String(s.to_string()))
    }
}

impl std::fmt::Display for RDLTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RDLTypes::String(s) => write!(f, "{s}"),
            RDLTypes::Number(n) => write!(f, "{n}"),
            RDLTypes::Vector(v) => write!(f, "{v:?}"),
            RDLTypes::Boolean(b) => write!(f, "{b}"),
        }
    }
}