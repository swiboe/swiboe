// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

/// Errors for use with Swiboe.

use mio;
use serde_json;
use std::error;
use std::fmt;
use std::io;
use std::result;
use std::sync::mpsc;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Disconnected,
    Io(io::Error),
    JsonParsing(serde_json::error::Error),
    RpcDone,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        error::Error::description(&*self).fmt(f)
    }
}

impl error::Error for Error {
  fn description(&self) -> &str {
      match *self {
          Error::Disconnected => "Channel or Socket is disconnected.",
          Error::Io(ref e) => e.description(),
          Error::JsonParsing(ref e) => e.description(),
          Error::RpcDone => "RPC is already finished or cancelled.",
      }
  }

  fn cause(&self) -> Option<&error::Error> {
      match *self {
          Error::Io(ref e) => Some(e),
          Error::JsonParsing(ref e) => Some(e),
          _ => None,
      }
  }
}

impl From<io::Error> for Error {
     fn from(error: io::Error) -> Self {
         Error::Io(error)
     }
}

impl<T> From<mpsc::SendError<T>> for Error {
    fn from(_: mpsc::SendError<T>) -> Self {
        Error::Disconnected
    }
}

impl From<mpsc::RecvError> for Error {
     fn from(_: mpsc::RecvError) -> Self {
         Error::Disconnected
     }
}

impl From<serde_json::error::Error> for Error {
     fn from(error: serde_json::error::Error) -> Self {
         Error::JsonParsing(error)
     }
}

impl<T> From<mio::NotifyError<T>> for Error {
    fn from(_: mio::NotifyError<T>) -> Self {
        Error::Disconnected
    }
}
