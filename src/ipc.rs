// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::Result;
use ::rpc;
use mio::{TryRead, TryWrite};
use serde_json;
use std::error::Error;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    RpcCall(rpc::Call),
    RpcResponse(rpc::Response),
    RpcCancel(rpc::Cancel),
}

pub struct Reader<T: Read> {
    pub socket: T,
    read_buffer: Vec<u8>,
    size_buffer: [u8; 4],
}

impl<T: Read> Reader<T> {
    pub fn new(socket: T) -> Self {
        Reader {
            socket: socket,
            read_buffer: Vec::with_capacity(1024),
            size_buffer: [0; 4],
        }
    }

    // NOCOM(#sirver): simplyfy code.
    pub fn read_one_message(&mut self) -> Result<Message> {
        try!(self.socket.read_exact(&mut self.size_buffer));
        let msg_len =
            ((self.size_buffer[3] as usize) << 24) |
            ((self.size_buffer[2] as usize) << 16) |
            ((self.size_buffer[1] as usize) <<  8) |
            ((self.size_buffer[0] as usize) <<  0);

        self.read_buffer.reserve(msg_len);
        unsafe {
            self.read_buffer.set_len(msg_len);
        }
        try!(self.socket.read_exact(&mut self.read_buffer));

        // NOCOM(#sirver): this should not unwrap.
        let msg = String::from_utf8(self.read_buffer.drain(..msg_len).collect()).unwrap();
        let message: Message = try!(serde_json::from_str(&msg));
        return Ok(message)
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
    // The number of bytes already written in to_write[0]. Once all are written, to_write[0] is
    // popped.
    num_written: usize,
    to_write: Vec<Vec<u8>>,
    pub socket: T,
}

pub enum WriterState {
    MoreToWrite,
    AllWritten,
}

impl<T: Write> Writer<T> {
    pub fn new(socket: T) -> Self {
        Writer {
            socket: socket,
            num_written: 0,
            to_write: Vec::new(),
        }
    }

    pub fn queue_message(&mut self, message: &Message) {
        let buffer = serde_json::to_vec(message).unwrap();

        let len = vec![
            (buffer.len() >> 0) as u8,
            (buffer.len() >> 8) as u8,
            (buffer.len() >> 16) as u8,
            (buffer.len() >> 24) as u8 ];
        self.to_write.push(len);
        self.to_write.push(buffer);
    }

    pub fn try_write(&mut self) -> Result<WriterState> {
        if self.to_write.is_empty() {
            return Ok(WriterState::AllWritten);
        }

        if let Some(num_written) = try!(self.socket.try_write(&self.to_write[0][self.num_written..])) {
            self.num_written += num_written;
        }

        if self.num_written == self.to_write[0].len() {
            self.to_write.remove(0);
            self.num_written = 0;
            self.try_write()
        } else {
            Ok(WriterState::MoreToWrite)
        }
    }
}
