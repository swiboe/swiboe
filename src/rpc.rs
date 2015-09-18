// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use serde;
use serde_json;
use std::error::Error as StdError;

// NOCOM(#sirver): add documentation (using this lint that forbids not having documentation).
//
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ResponseKind {
    Last(Result),
    Partial(serde_json::Value),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub context: String,
    pub kind: ResponseKind,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamingResult {
    pub context: String,
    pub value: serde_json::Value,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ErrorKind {
    UnknownRpc,
    Io,
    InvalidArgs,
}

impl ErrorKind {
    pub fn from_str(description: &str) -> ErrorKind {
        match description {
            "UnknownRpc" => ErrorKind::UnknownRpc,
            "Io" => ErrorKind::Io,
            "InvalidArgs" => ErrorKind::InvalidArgs,
            _ => panic!("{} is not a valid ErrorKind name.", description),
        }
    }

    pub fn to_str(&self) -> &str {
        match *self {
            ErrorKind::UnknownRpc => "UnknownRpc",
            ErrorKind::Io => "Io",
            ErrorKind::InvalidArgs => "InvalidArgs",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub details: Option<serde_json::Value>,
}

impl From<serde_json::error::Error> for Error {
     fn from(error: serde_json::error::Error) -> Self {
         Error {
             kind: ErrorKind::InvalidArgs,
             details: Some(serde_json::to_value(&error.description())),
         }
     }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Result {
    // NOCOM(#sirver): mention success as a convenient creating for this.
    Ok(serde_json::Value),
    Err(Error),
    NotHandled,
}

impl Result {
    pub fn success<T: serde::Serialize>(value: T) -> Result {
        Result::Ok(serde_json::to_value(&value))
    }

    pub fn unwrap_err(self) -> Error {
        use self::Result::*;
        match self {
            Ok(_) | NotHandled => panic!("Called unwrap_rpc_error on a non_error."),
            Err(e) => e,
        }
    }

    pub fn is_ok(&self) -> bool {
        if let &Result::Ok(_) = self {
            true
        } else {
            false
        }
    }
}

// NOCOM(#sirver): check in this file what needs to be derived. seems too much.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Call {
    pub function: String,
    pub context: String,
    pub args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cancel {
    pub context: String,
}
