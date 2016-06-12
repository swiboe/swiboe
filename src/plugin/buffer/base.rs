// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use ::client::RpcCaller;
use ::client;
use ::rpc;
use serde_json;
use std::collections::HashMap;
use std::io;
use std::ops;
use std::result;
use std::string;

#[derive(Debug)]
pub enum BufferError {
    UnknownBuffer,
}

impl From<BufferError> for rpc::Error {
     fn from(error: BufferError) -> Self {
         use rpc::ErrorKind::*;

         let (kind, details) = match error {
             BufferError::UnknownBuffer => (InvalidArgs, format!("unknown_buffer")),
         };

         rpc::Error {
             kind: kind,
             details: Some(serde_json::to_value(&details)),
         }
     }
}

impl From<io::Error> for rpc::Error {
     fn from(error: io::Error) -> Self {
         let details = match error.kind() {
             io::ErrorKind::NotFound => "not_found",
             io::ErrorKind::PermissionDenied => "not_found",
             io::ErrorKind::ConnectionRefused => "connection_refused",
             io::ErrorKind::ConnectionReset => "connection_reset",
             io::ErrorKind::ConnectionAborted => "connection_aborted",
             io::ErrorKind::NotConnected => "not_connected",
             io::ErrorKind::AddrInUse => "addr_in_use",
             io::ErrorKind::AddrNotAvailable => "addr_not_available",
             io::ErrorKind::BrokenPipe => "broken_pipe",
             io::ErrorKind::AlreadyExists => "already_exists",
             io::ErrorKind::WouldBlock => "would_block",
             io::ErrorKind::InvalidInput => "invalid_input",
             io::ErrorKind::InvalidData => "invalid_data",
             io::ErrorKind::TimedOut => "timed_out",
             io::ErrorKind::WriteZero => "write_zero",
             io::ErrorKind::Interrupted => "interrupted",
             _ => "unknown",
         };
         rpc::Error {
             kind: rpc::ErrorKind::Io,
             details: Some(serde_json::to_value(&details)),
         }
     }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BufferCreated {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BufferDeleted {
    pub buffer_index: usize,
}

pub struct Buffer {
    // TODO(sirver): This should probably be something more clever, like a rope or a gap buffer.
    content: String,
}

impl string::ToString for Buffer {
    fn to_string(&self) -> String {
        self.content.clone()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self::from_string("".into())
    }

    pub fn from_string(content: String) -> Self {
        Buffer {
            content: content,
        }
    }
}

pub struct BuffersManager {
    next_buffer_index: usize,
    buffers: HashMap<usize, Buffer>,
    client: client::ThinClient,
}

impl BuffersManager {
    pub fn new(client: client::ThinClient) -> Self {
        BuffersManager {
            next_buffer_index: 0,
            buffers: HashMap::new(),
            client: client
        }
    }

    pub fn new_buffer(&mut self, buffer: Buffer) -> usize {
        let current_buffer_index = self.next_buffer_index;
        self.next_buffer_index += 1;

        self.buffers.insert(current_buffer_index, buffer);

        // NOCOM(#sirver): new is not good. should be create.
        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.client.call("on.buffer.new", &BufferCreated {
            buffer_index: current_buffer_index,
        });
        current_buffer_index
    }

    pub fn delete_buffer(&mut self, buffer_index: usize) -> result::Result<(), BufferError> {
        try!(self.buffers.remove(&buffer_index).ok_or(BufferError::UnknownBuffer));

        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.client.call("on.buffer.deleted", &BufferDeleted {
            buffer_index: buffer_index,
        });

        Ok(())
    }

    pub fn get(&self, index: usize) -> result::Result<&Buffer, BufferError> {
        let buffer = try!(self.buffers.get(&index).ok_or(BufferError::UnknownBuffer));
        Ok(buffer)
    }
}

impl ops::Deref for BuffersManager {
    type Target = HashMap<usize, Buffer>;

    fn deref(&self) -> &HashMap<usize, Buffer> {
        &self.buffers
    }
}

