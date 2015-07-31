use super::{Error, ErrorKind, Result};
use std::io::{self, Read, Write};

// NOCOM(#sirver): add documentation (using this lint that forbids not having documentation).

pub trait IpcRead {
    fn read_message(&mut self, s: &mut Vec<u8>) -> Result<()>;
}

pub trait IpcWrite {
    fn write_message(&mut self, s: &[u8]) -> Result<()>;
}

impl<T: Read> IpcRead for T {
    fn read_message(&mut self, s: &mut Vec<u8>) -> Result<()> {
        let mut length: [u8; 4] = [0, 0, 0, 0];
        let nread = try!(self.read(&mut length));
        if nread != 4 {
            return Err(Error::new(ErrorKind::Io(
                        io::Error::new(io::ErrorKind::Other, "Did not read 4 bytes."))));
        }

        let size =
            ((length[3] as usize) << 24) |
            ((length[2] as usize) << 16) |
            ((length[1] as usize) <<  8) |
            ((length[0] as usize) <<  0);

        s.reserve(size);
        try!(self.take(size as u64).read_to_end(s));
        Ok(())
    }
}

impl<T: Write> IpcWrite for T {
    fn write_message(&mut self, s: &[u8]) -> Result<()> {
        let len = [
            (s.len() >> 0) as u8,
            (s.len() >> 8) as u8,
            (s.len() >> 16) as u8,
            (s.len() >> 24) as u8 ];

        try!(self.write_all(&len));
        try!(self.write_all(&s));
        Ok(())
    }
}
