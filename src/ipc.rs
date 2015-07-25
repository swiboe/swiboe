use std::io::{Read, Write};

// NOCOM(#sirver): add error handling
pub trait IpcRead {
    fn read_message(&mut self, s: &mut Vec<u8>);
}

pub trait IpcWrite {
    fn write_message(&mut self, s: &[u8]);
}

impl<T: Read> IpcRead for T {
    fn read_message(&mut self, s: &mut Vec<u8>) {
        let mut length: [u8; 4] = [0, 0, 0, 0];
        // NOCOM(#sirver): make sure this read 4 bytes
        self.read(&mut length).unwrap();

        let size = ((length[3] as usize) << 24) |
            ((length[2] as usize) << 16) |
            ((length[1] as usize) <<  8) |
            ((length[0] as usize) <<  0);

        s.reserve(size);
        self.take(size as u64).read_to_end(s).unwrap();
    }
}

impl<T: Write> IpcWrite for T {
    fn write_message(&mut self, s: &[u8]) {
        let len = [
            (s.len() >> 0) as u8,
            (s.len() >> 8) as u8,
            (s.len() >> 16) as u8,
            (s.len() >> 24) as u8 ];

        self.write_all(&len).unwrap();
        self.write_all(&s).unwrap();
    }
}
