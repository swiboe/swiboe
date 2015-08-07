use serde::json;
use std::collections::HashMap;
use std::convert;
use std::path;
use std::string;
use std::sync::{RwLock, Arc};
use super::client;
use super::ipc;

// NOCOM(#sirver): make a new package rpc and move some stuff itno that?
struct New {
    buffers: Arc<RwLock<BuffersManager>>,
}

#[derive(Debug)]
pub enum BufferError {
    UnknownBuffer,
}

impl From<BufferError> for ipc::RpcError {
     fn from(error: BufferError) -> Self {
         use ipc::RpcErrorKind::*;

         let (kind, details) = match error {
             BufferError::UnknownBuffer => (InvalidArgs, format!("unknown_buffer")),
         };

         ipc::RpcError {
             kind: kind,
             details: Some(json::to_value(&details)),
         }
         // NOCOM(#sirver): more information!
     }
}


// NOCOM(#sirver): kill?
macro_rules! try_rpc {
    ($expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => {
            return ipc::RpcResult::Err(convert::From::from(err))
        }
    })
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BufferCreated {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BufferDeleted {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewRequest {
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewResponse {
    pub buffer_index: usize,
}

impl client::RemoteProcedure for New {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        // NOCOM(#sirver): need testing for bad request results
        // NOCOM(#sirver): needs some understanding what happens on extra values.
        let request: NewRequest = try_rpc!(json::from_value(args));
        let mut buffers = self.buffers.write().unwrap();

        let buffer = match request.content {
            Some(content) => Buffer::from_string(content),
            None => Buffer::new(),
        };

        let response = NewResponse {
            buffer_index: buffers.new_buffer(buffer),
        };
        ipc::RpcResult::success(response)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DeleteRequest {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DeleteResponse;

struct Delete {
    buffers: Arc<RwLock<BuffersManager>>,
}

impl client::RemoteProcedure for Delete {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        // NOCOM(#sirver): needs some understanding what happens on extra values.
        let request: DeleteRequest = try_rpc!(json::from_value(args));
        let mut buffers = self.buffers.write().unwrap();
        // NOCOM(#sirver): handle errors
        try_rpc!(buffers.delete_buffer(request.buffer_index));

        let response = DeleteResponse;
        ipc::RpcResult::success(response)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GetContentRequest {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct GetContentResponse {
    pub content: String,
}

struct GetContent {
    buffers: Arc<RwLock<BuffersManager>>,
}

impl client::RemoteProcedure for GetContent {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        let request: GetContentRequest = try_rpc!(json::from_value(args));
        let buffers = self.buffers.read().unwrap();

        let buffer = try_rpc!(buffers.get(request.buffer_index));

        let response = GetContentResponse {
            content: buffer.to_string(),
        };
        ipc::RpcResult::success(response)
    }
}

struct Buffer {
    // TODO(sirver): This should probably be something more clever, like a rope or a gap buffer.
    content: String,
}

impl string::ToString for Buffer {
    fn to_string(&self) -> String {
        self.content.clone()
    }
}

impl Buffer {
    fn new() -> Self {
        Self::from_string("".into())
    }

    fn from_string(content: String) -> Self {
        Buffer {
            content: content,
        }
    }
}

struct BuffersManager {
    next_buffer_index: usize,
    buffers: HashMap<usize, Buffer>,
    rpc_caller: client::RpcCaller,
}

impl BuffersManager {
    fn new(rpc_caller: client::RpcCaller) -> Self {
        BuffersManager {
            next_buffer_index: 0,
            buffers: HashMap::new(),
            rpc_caller: rpc_caller
        }
    }

    fn new_buffer(&mut self, buffer: Buffer) -> usize {
        let current_buffer_index = self.next_buffer_index;
        self.next_buffer_index += 1;

        self.buffers.insert(current_buffer_index, buffer);

        // NOCOM(#sirver): new is not good. should be create.
        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.rpc_caller.call("on.buffer.new", &BufferCreated {
            buffer_index: current_buffer_index,
        });
        current_buffer_index
    }

    fn delete_buffer(&mut self, buffer_index: usize) -> Result<(), BufferError> {
        try!(self.buffers.remove(&buffer_index).ok_or(BufferError::UnknownBuffer));

        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.rpc_caller.call("on.buffer.deleted", &BufferDeleted {
            buffer_index: buffer_index,
        });

        Ok(())
    }

    fn get(&self, index: usize) -> Result<&Buffer, BufferError> {
        let buffer = try!(self.buffers.get(&index).ok_or(BufferError::UnknownBuffer));
        Ok(buffer)
    }
}

pub struct BufferPlugin<'a> {
    client: client::Client<'a>,
    buffers: Arc<RwLock<BuffersManager>>,
}

impl<'a> BufferPlugin<'a> {
    pub fn new(socket_name: &path::Path) -> Self {
        let client = client::Client::connect(socket_name);

        let plugin = BufferPlugin {
            buffers: Arc::new(RwLock::new(BuffersManager::new(client.new_rpc_caller()))),
            client: client,
        };

        let new = Box::new(New { buffers: plugin.buffers.clone() });
        plugin.client.new_rpc("buffer.new", new);

        let delete = Box::new(Delete { buffers: plugin.buffers.clone() });
        plugin.client.new_rpc("buffer.delete", delete);

        let get_content = Box::new(GetContent { buffers: plugin.buffers.clone() });
        plugin.client.new_rpc("buffer.get_content", get_content);

        plugin
    }
}
