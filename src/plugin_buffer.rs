use serde::json;
use std::collections::HashMap;
use std::io::{self, Read};
use std::convert;
use std::fs;
use std::path;
use std::string;
use std::sync::{RwLock, Arc};
use super::client;
use super::ipc;

// NOCOM(#sirver): make a new package rpc and move some stuff itno that?
struct New {
    // NOCOM(#sirver): is the Arc needed? Maybe we can pass a reference to all the rpcs.
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
     }
}

impl From<io::Error> for ipc::RpcError {
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
         ipc::RpcError {
             kind: ipc::RpcErrorKind::Io,
             details: Some(json::to_value(&details)),
         }
     }
}

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

// NOCOM(#sirver): what does serde do if there are extra values in the JSON?
impl client::RemoteProcedure for New {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        // NOCOM(#sirver): need testing for bad request results
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
        let request: DeleteRequest = try_rpc!(json::from_value(args));
        let mut buffers = self.buffers.write().unwrap();
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct OpenResponse {
    pub buffer_index: usize,
}

struct Open {
    buffers: Arc<RwLock<BuffersManager>>,
}

impl client::RemoteProcedure for Open {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        const FILE_PREFIX: &'static str = "file://";
        let mut request: OpenRequest = try_rpc!(json::from_value(args));
        if !request.uri.starts_with(FILE_PREFIX) {
            return ipc::RpcResult::NotHandled;
        }
        request.uri.drain(..FILE_PREFIX.len());

        let mut file = try_rpc!(fs::File::open(path::Path::new(&request.uri)));
        let mut content = String::new();
        try_rpc!(file.read_to_string(&mut content));

        let buffer = Buffer::from_string(content);

        let mut buffers = self.buffers.write().unwrap();
        let response = OpenResponse {
            buffer_index: buffers.new_buffer(buffer),
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

        let open = Box::new(Open { buffers: plugin.buffers.clone() });
        plugin.client.new_rpc("buffer.open", open);

        plugin
    }
}
