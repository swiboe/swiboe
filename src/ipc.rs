use libc::consts::os::posix88;
use serde::{self, json};
use std::io::{self, Read, Write};
use super::Result;

// NOCOM(#sirver): add documentation (using this lint that forbids not having documentation).


// NOCOM(#sirver): I think this can be killed.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RpcState {
    Running,
    Done,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcReply {
    pub context: String,
    pub state: RpcState,
    pub result: RpcResult,
}

// NOCOM(#sirver): more compact custom serialization?
// NOCOM(#sirver): use actual result type?
// NOCOM(#sirver): remove *Kind? at the end
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RpcResult {
    // NOCOM(#sirver): mention success as a convenient creating for this.
    Ok(json::Value),
    NotHandled,
}

impl RpcResult {
    pub fn success<T: serde::Serialize>(value: T) -> RpcResult {
        RpcResult::Ok(json::to_value(&value))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RpcCall {
    pub function: String,
    pub context: String,
    pub args: json::Value,
}

// NOCOM(#sirver): most of the entries here could be Cow.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    RpcCall(RpcCall),
    RpcReply(RpcReply),
    Broadcast(json::Value),
}

// NOCOM(#sirver): these could deal directly with thes structs.
pub trait IpcRead {
    fn read_message(&mut self) -> Result<Message>;
}

pub trait IpcWrite {
    fn write_message(&mut self, message: &Message) -> Result<()>;
}

// TODO(hrapp): This kinda defeats the purpose of MIO a bit, but it makes it very convenient to
// read always a full message. No buffering of our own is needed then.
fn read_all<T: Read>(reader: &mut T, buffer: &mut [u8]) -> io::Result<()> {
    let mut num_read = 0;
    while num_read < buffer.len() {
        match reader.read(&mut buffer[num_read..]) {
            Ok(len) => num_read += len,
            Err(err) => {
                if err.raw_os_error() != Some(posix88::EAGAIN) {
                    return Err(err);
                }
            }
        }
    }
    Ok(())
}

impl<T: Read> IpcRead for T {
    fn read_message(&mut self) -> Result<Message> {
        let mut length: [u8; 4] = [0, 0, 0, 0];
        try!(read_all(self, &mut length));

        let mut buffer = Vec::<u8>::new();

        let size =
            ((length[3] as usize) << 24) |
            ((length[2] as usize) << 16) |
            ((length[1] as usize) <<  8) |
            ((length[0] as usize) <<  0);

        // NOCOM(#sirver): this can skip the buffer
        buffer.reserve(size);
        unsafe {
            buffer.set_len(size);
        }
        try!(read_all(self, &mut buffer));

        // NOCOM(#sirver): read directly into this string.
        let msg = String::from_utf8(buffer).unwrap();

        let message: Message = try!(json::from_str(&msg));
        Ok(message)
    }
}

impl<T: Write> IpcWrite for T {
    fn write_message(&mut self, message: &Message) -> Result<()> {
        // NOCOM(#sirver): is that maximal efficient?
        let buffer = json::to_string(message).unwrap();
        let len = [
            (buffer.len() >> 0) as u8,
            (buffer.len() >> 8) as u8,
            (buffer.len() >> 16) as u8,
            (buffer.len() >> 24) as u8 ];

        try!(self.write_all(&len));
        try!(self.write_all(buffer.as_bytes()));
        Ok(())
    }
}
