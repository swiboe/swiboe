use libc::consts::os::posix88;
use mio::{TryRead};
use serde::{self, json};
use std::error::Error;
use std::io::{self, Read, Write};
use super::Result;

// NOCOM(#sirver): add documentation (using this lint that forbids not having documentation).

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcResponse {
    pub context: String,
    pub result: RpcResult,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum RpcErrorKind {
    UnknownRpc,
    Io,
    InvalidArgs,
}

// TODO(sirver): To get really nice looking JSON, a lot of this serialization has to be tweaked.
// Maybe ask on the serde tracker how to get beautiful serialization. This is here just to
// understand how it would work. Maybe remove in the future?
// impl serde::Serialize for RpcErrorKind {
    // fn serialize<S>(&self, serializer: &mut S) -> result::Result<(), S::Error>
        // where S: serde::Serializer,
    // {
        // let s = match *self {
            // RpcErrorKind::UnknownRpc => "unknown_rpc",
            // RpcErrorKind::InvalidArgs => "invalid_args",
            // RpcErrorKind::Io => "io",
        // };
        // serializer.visit_str(s)
    // }
// }

// struct RpcErrorKindVisitor;

// impl serde::de::Visitor for RpcErrorKindVisitor {
    // type Value = RpcErrorKind;

    // fn visit_str<E>(&mut self, value: &str) -> result::Result<RpcErrorKind, E>
        // where E: serde::de::Error
    // {
        // match value {
            // "unknown_rpc" => Ok(RpcErrorKind::UnknownRpc),
            // "invalid_args" => Ok(RpcErrorKind::InvalidArgs),
            // _ => Err(serde::de::Error::unknown_field_error(value)),
        // }
    // }
// }

// impl serde::Deserialize for RpcErrorKind {
    // fn deserialize<D>(deserializer: &mut D) -> result::Result<Self, D::Error>
        // where D: serde::de::Deserializer {
        // deserializer.visit(RpcErrorKindVisitor)
    // }
// }

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RpcError {
    pub kind: RpcErrorKind,
    pub details: Option<json::Value>,
}

impl From<json::error::Error> for RpcError {
     fn from(error: json::error::Error) -> Self {
         RpcError {
             kind: RpcErrorKind::InvalidArgs,
             details: Some(json::to_value(&error.description())),
         }
     }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RpcResult {
    // NOCOM(#sirver): mention success as a convenient creating for this.
    Ok(json::Value),
    Err(RpcError),
    // NOCOM(#sirver): Not Handled is never seen by a client, but is send by one.
    NotHandled,
}

impl RpcResult {
    pub fn success<T: serde::Serialize>(value: T) -> RpcResult {
        RpcResult::Ok(json::to_value(&value))
    }

    pub fn unwrap_err(self) -> RpcError {
        match self {
            RpcResult::Ok(_) | RpcResult::NotHandled => panic!("Called unwrap_rpc_error on a non_error."),
            RpcResult::Err(e) => e,
        }
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
    RpcResponse(RpcResponse),
}


// TODO(hrapp): This kinda defeats the purpose of MIO a bit, but it makes it very convenient to
// read always a full message. No buffering of our own is needed then. This might impact
// performance really negatively and might also lead to deadlooks, so we should get rid of it AFAP.
// mio provides some buffers that look usefull.
fn write_all<T: Write>(writer: &mut T, buffer: &[u8]) -> io::Result<()> {
    let mut num_written = 0;
    while num_written < buffer.len() {
        match writer.write(&buffer[num_written..]) {
            Ok(len) => num_written += len,
            Err(err) => {
                if err.raw_os_error() != Some(posix88::EAGAIN) {
                    return Err(err);
                }
                println!("#sirver write EAGAIN");
            }
        }
    }
    Ok(())
}

pub struct IpcStream<T: Read> {
    pub socket: T,
    read_buffer: Vec<u8>,
}

impl<T: Read + Write> IpcStream<T> {
    pub fn new(socket: T) -> Self {
        IpcStream {
            socket: socket,
            read_buffer: Vec::with_capacity(1024),
        }
    }

    pub fn read_message(&mut self) -> Result<Option<Message>> {
        try!(self.socket.try_read_buf(&mut self.read_buffer));

        if self.read_buffer.len() < 4 {
            return Ok(None);
        }

        let msg_len =
            ((self.read_buffer[3] as usize) << 24) |
            ((self.read_buffer[2] as usize) << 16) |
            ((self.read_buffer[1] as usize) <<  8) |
            ((self.read_buffer[0] as usize) <<  0);

        if self.read_buffer.len() < msg_len + 4 {
            return Ok(None);
        }

        // NOCOM(#sirver): this should not unwrap.
        let msg = String::from_utf8(self.read_buffer.drain(..4+msg_len).skip(4).collect()).unwrap();
        let message: Message = try!(json::from_str(&msg));
        return Ok(Some(message))
    }

    pub fn write_message(&mut self, message: &Message) -> Result<()> {
        // NOCOM(#sirver): is that maximal efficient?
        let buffer = json::to_string(message).unwrap();
        let len = [
            (buffer.len() >> 0) as u8,
            (buffer.len() >> 8) as u8,
            (buffer.len() >> 16) as u8,
            (buffer.len() >> 24) as u8 ];

        try!(write_all(&mut self.socket, &len));
        try!(write_all(&mut self.socket, buffer.as_bytes()));
        Ok(())
    }
}
