// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use mio::{TryRead, TryWrite};
use rpc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::{Read, Write};
use Result;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    RpcCall(rpc::Call),
    RpcResponse(rpc::Response),
    RpcCancel(rpc::Cancel),
}

pub struct Reader<T: Read> {
    pub socket: T,
    buffer: Vec<u8>,
}

fn parse_length(buf: &[u8]) -> usize {
    ((buf[3] as usize) << 24)
        | ((buf[2] as usize) << 16)
        | ((buf[1] as usize) << 8)
        | ((buf[0] as usize) << 0)
}

// Tries to parse a JSON message into an ipc::Message struct.
fn to_message(data: &[u8]) -> Result<Message> {
    let message: Message = serde_json::from_slice(data)?;
    return Ok(message);
}

impl<T: Read> Reader<T> {
    pub fn new(socket: T) -> Self {
        Reader {
            socket: socket,
            buffer: Vec::with_capacity(1024),
        }
    }

    /// Read one full message - this expects the underlying socket to be blocking.
    pub fn read_message(&mut self) -> Result<Message> {
        let mut size_buf = [0u8; 4];
        self.socket.read_exact(&mut size_buf)?;
        let msg_len = parse_length(&size_buf);

        self.buffer.reserve(msg_len);
        unsafe {
            self.buffer.set_len(msg_len);
        }
        self.socket.read_exact(&mut self.buffer)?;
        to_message(&self.buffer)
    }

    /// Read all data currently available on the socket and returns the next full message that is
    /// available or None if there is no full one.
    pub fn try_read_message(&mut self) -> Result<Option<Message>> {
        // This might reallocate 'buffer' if it is too small.
        self.socket.try_read_buf(&mut self.buffer)?;
        if self.buffer.len() < 4 {
            return Ok(None);
        }

        let msg_len = parse_length(&self.buffer[..4]);
        if self.buffer.len() < msg_len + 4 {
            return Ok(None);
        }
        let message = to_message(&self.buffer[4..4 + msg_len]);
        self.buffer.drain(..4 + msg_len);

        message.map(|message| Some(message))
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

fn encode(message: &Message) -> Result<(Vec<u8>, Vec<u8>)> {
    let buffer = serde_json::to_vec(message)?;
    let len = vec![
        (buffer.len() >> 0) as u8,
        (buffer.len() >> 8) as u8,
        (buffer.len() >> 16) as u8,
        (buffer.len() >> 24) as u8,
    ];
    Ok((len, buffer))
}

impl<T: Write> Writer<T> {
    pub fn new(socket: T) -> Self {
        Writer {
            socket: socket,
            num_written: 0,
            to_write: Vec::new(),
        }
    }

    pub fn write_message(&mut self, message: &Message) -> Result<()> {
        let (len, buffer) = encode(message)?;
        self.socket.write_all(&len)?;
        self.socket.write_all(&buffer)?;
        Ok(())
    }

    pub fn queue_message(&mut self, message: &Message) {
        // NOCOM(#sirver): should not unwrap
        let (len, buffer) = encode(message).unwrap();
        self.to_write.push(len);
        self.to_write.push(buffer);
    }

    pub fn try_write(&mut self) -> Result<WriterState> {
        if self.to_write.is_empty() {
            return Ok(WriterState::AllWritten);
        }

        if let Some(num_written) = self
            .socket
            .try_write(&self.to_write[0][self.num_written..])?
        {
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
