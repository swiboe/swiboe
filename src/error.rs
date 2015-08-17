/// Errors for use with Switchboard.

use serde_json;
use std::error;
use std::fmt;
use std::io;
use std::result;
use std::sync::mpsc;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum ErrorKind {
    ClientDisconnected,
    Disconnected(mpsc::RecvError),
    Io(io::Error),
    JsonParsing(serde_json::error::Error),
}

// NOCOM(#sirver): kill and just use the enum
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Error {
            kind: kind,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        error::Error::description(&*self).fmt(f)
    }
}

impl error::Error for Error {
  fn description(&self) -> &str {
      match self.kind {
          ErrorKind::ClientDisconnected => "Client disconnected.",
          ErrorKind::Disconnected(_) => "Channel is disconnected.",
          ErrorKind::Io(ref e) => e.description(),
          ErrorKind::JsonParsing(ref e) => e.description(),
      }
  }

  fn cause(&self) -> Option<&error::Error> {
      match self.kind {
          ErrorKind::ClientDisconnected => None,
          ErrorKind::Disconnected(ref e) => Some(e),
          ErrorKind::Io(ref e) => Some(e),
          ErrorKind::JsonParsing(ref e) => Some(e),
      }
  }
}

impl From<io::Error> for Error {
     fn from(error: io::Error) -> Self {
         Error::new(ErrorKind::Io(error))
     }
}

impl From<mpsc::RecvError> for Error {
     fn from(error: mpsc::RecvError) -> Self {
         Error::new(ErrorKind::Disconnected(error))
     }
}

impl From<serde_json::error::Error> for Error {
     fn from(error: serde_json::error::Error) -> Self {
         Error::new(ErrorKind::JsonParsing(error))
     }
}
