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

// TODO(sirver): To get really nice looking JSON, a lot of this serialization has to be tweaked.
// Maybe ask on the serde tracker how to get beautiful serialization. This is here just to
// understand how it would work. Maybe remove in the future?
// impl serde::Serialize for ErrorKind {
    // fn serialize<S>(&self, serializer: &mut S) -> result::Result<(), S::Error>
        // where S: serde::Serializer,
    // {
        // let s = match *self {
            // ErrorKind::Unknown => "unknown_rpc",
            // ErrorKind::InvalidArgs => "invalid_args",
            // ErrorKind::Io => "io",
        // };
        // serializer.visit_str(s)
    // }
// }

// struct ErrorKindVisitor;

// impl serde::de::Visitor for ErrorKindVisitor {
    // type Value = ErrorKind;

    // fn visit_str<E>(&mut self, value: &str) -> result::Result<ErrorKind, E>
        // where E: serde::de::Error
    // {
        // match value {
            // "unknown_rpc" => Ok(ErrorKind::UnknownRpc),
            // "invalid_args" => Ok(ErrorKind::InvalidArgs),
            // _ => Err(serde::de::Error::unknown_field_error(value)),
        // }
    // }
// }

// impl serde::Deserialize for ErrorKind {
    // fn deserialize<D>(deserializer: &mut D) -> result::Result<Self, D::Error>
        // where D: serde::de::Deserializer {
        // deserializer.visit(ErrorKindVisitor)
    // }
// }

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
