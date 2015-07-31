/// Errors for use with Switchboard.

use std::error;
use std::fmt;
use std::io;
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum ErrorKind {
    Io(io::Error),
}

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
          ErrorKind::Io(ref e) => e.description(),
      }
  }

  fn cause(&self) -> Option<&error::Error> {
      match self.kind {
          ErrorKind::Io(ref e) => Some(e),
      }
  }
}

impl From<io::Error> for Error {
     fn from(error: io::Error) -> Self {
         Error::new(ErrorKind::Io(error))
     }
}
