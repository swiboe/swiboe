use serde::{self, json};
use std::error::Error as StdError;

// NOCOM(#sirver): add documentation (using this lint that forbids not having documentation).
//
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ResponseKind {
    Last(Result),
    Partial(json::Value),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub context: String,
    pub kind: ResponseKind,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamingResult {
    pub context: String,
    pub value: json::Value,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ErrorKind {
    UnknownRpc,
    Io,
    InvalidArgs,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub details: Option<json::Value>,
}

impl From<json::error::Error> for Error {
     fn from(error: json::error::Error) -> Self {
         Error {
             kind: ErrorKind::InvalidArgs,
             details: Some(json::to_value(&error.description())),
         }
     }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Result {
    // NOCOM(#sirver): mention success as a convenient creating for this.
    Ok(json::Value),
    Err(Error),
    NotHandled,
}

impl Result {
    pub fn success<T: serde::Serialize>(value: T) -> Result {
        Result::Ok(json::to_value(&value))
    }

    pub fn unwrap_err(self) -> Error {
        use self::Result::*;
        match self {
            Ok(_) | NotHandled => panic!("Called unwrap_rpc_error on a non_error."),
            Err(e) => e,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Call {
    pub function: String,
    pub context: String,
    pub args: json::Value,
}
