use super::Result;
use std::io::{self, Read, Write};

// NOCOM(#sirver): add documentation (using this lint that forbids not having documentation).

pub trait IpcRead {
    fn read_message(&mut self, buffer: &mut Vec<u8>) -> Result<()>;
}

pub trait IpcWrite {
    fn write_message(&mut self, buffer: &[u8]) -> Result<()>;
}

fn read_all<T: Read>(reader: &mut T, buffer: &mut [u8]) -> io::Result<()> {
    let mut num_read = 0;
    while num_read < buffer.len() {
        match reader.read(&mut buffer[num_read..]) {
            Ok(len) => num_read += len,
            Err(err) => {
                // NOCOM(#sirver): hardly compatible
                if err.raw_os_error() != Some(35) {
                    return Err(err);
                }
            }
        }
    }
    Ok(())
}

impl<T: Read> IpcRead for T {
    fn read_message(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        let mut length: [u8; 4] = [0, 0, 0, 0];
        try!(read_all(self, &mut length));

        let size =
            ((length[3] as usize) << 24) |
            ((length[2] as usize) << 16) |
            ((length[1] as usize) <<  8) |
            ((length[0] as usize) <<  0);

        buffer.reserve(size);
        unsafe {
            buffer.set_len(size);
        }
        try!(read_all(self, buffer));
        Ok(())
    }
}

impl<T: Write> IpcWrite for T {
    fn write_message(&mut self, buffer: &[u8]) -> Result<()> {
        let len = [
            (buffer.len() >> 0) as u8,
            (buffer.len() >> 8) as u8,
            (buffer.len() >> 16) as u8,
            (buffer.len() >> 24) as u8 ];

        try!(self.write_all(&len));
        try!(self.write_all(&buffer));
        Ok(())
    }
}
