use libc::consts::os::posix88;
use serde::{self, json};
use std::io::{self, Read, Write};
use super::Result;
use std::error::Error;

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

pub trait IpcRead {
    fn read_message(&mut self) -> Result<Message>;
}

pub trait IpcWrite {
    fn write_message(&mut self, message: &Message) -> Result<()>;
}

// TODO(hrapp): This kinda defeats the purpose of MIO a bit, but it makes it very convenient to
// read always a full message. No buffering of our own is needed then. This might impact
// performance really negatively and might also lead to deadlooks, so we should get rid of it AFAP.
// mio provides some buffers that look usefull.
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

fn write_all<T: Write>(writer: &mut T, buffer: &[u8]) -> io::Result<()> {
    let mut num_written = 0;
    while num_written < buffer.len() {
        match writer.write(&buffer[num_written..]) {
            Ok(len) => num_written += len,
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

        try!(write_all(self, &len));
        try!(write_all(self, buffer.as_bytes()));
        Ok(())
    }
}
