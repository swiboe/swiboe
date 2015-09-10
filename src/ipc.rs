// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::Result;
use ::rpc;
use libc::consts::os::posix88;
use mio::{TryRead};
use serde_json;
use std::error::Error;
use std::io::{self, Read, Write};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    RpcCall(rpc::Call),
    RpcResponse(rpc::Response),
    RpcCancel(rpc::Cancel),
}


// TODO(hrapp): This kinda defeats the purpose of MIO a bit, but it makes it very convenient to
// read always a full message. No buffering of our own is needed then. This might impact
// performance really negatively and might also lead to deadlooks, so we should get rid of it AFAP.
// mio provides some buffers that look useful.
fn write_all<T: Write>(writer: &mut T, buffer: &[u8]) -> io::Result<()> {
    let mut num_written = 0;
    while num_written < buffer.len() {
        match writer.write(&buffer[num_written..]) {
            Ok(len) => num_written += len,
            Err(err) => {
                if err.raw_os_error() != Some(posix88::EAGAIN) {
                    return Err(err);
                }
                // println!("#sirver write EAGAIN");
            }
        }
    }
    Ok(())
}

pub struct Reader<T: Read> {
    pub socket: T,
    read_buffer: Vec<u8>,
}

impl<T: Read> Reader<T> {
    pub fn new(socket: T) -> Self {
        Reader {
            socket: socket,
            read_buffer: Vec::with_capacity(1024),
        }
    }

    pub fn read_message(&mut self) -> Result<Option<Message>> {
        // This might reallocate 'read_buffer' if it is too small.
        try!(self.socket.try_read_buf(&mut self.read_buffer));

        // We have read less than 4 bytes. We have to wait for more data to arrive.
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
        let message: Message = try!(serde_json::from_str(&msg));
        return Ok(Some(message))
    }
}

pub struct Writer<T: Write> {
    pub socket: T,
}


impl<T: Write> Writer<T> {
    pub fn new(socket: T) -> Self {
        Writer {
            socket: socket,
        }
    }

    pub fn write_message(&mut self, message: &Message) -> Result<()> {
        // NOCOM(#sirver): is that maximal efficient?
        let buffer = serde_json::to_string(message).unwrap();
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
