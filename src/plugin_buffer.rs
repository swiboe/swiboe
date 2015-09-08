use ::client;
use ::rpc;
use serde_json;
use std::collections::HashMap;
use std::convert;
use std::fs;
use std::io::{self, Read};
use std::ops;
use std::path;
use std::string;
use std::sync::{RwLock, Arc};

struct New {
    buffers: Arc<RwLock<BuffersManager>>,
}

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewRequest {
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewResponse {
    pub buffer_index: usize,
}

// NOCOM(#sirver): what does serde do if there are extra values in the JSON?
impl client::rpc::server::Rpc for New {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        // NOCOM(#sirver): need testing for bad request results
        let request: NewRequest = try_rpc!(context, serde_json::from_value(args));
        let mut buffers = self.buffers.write().unwrap();

        let buffer = match request.content {
            Some(content) => Buffer::from_string(content),
            None => Buffer::new(),
        };

        let response = NewResponse {
            buffer_index: buffers.new_buffer(buffer),
        };
        context.finish(rpc::Result::success(response)).unwrap();
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

impl client::rpc::server::Rpc for Delete {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: DeleteRequest = try_rpc!(context, serde_json::from_value(args));
        let mut buffers = self.buffers.write().unwrap();
        try_rpc!(context, buffers.delete_buffer(request.buffer_index));

        let response = DeleteResponse;
        context.finish(rpc::Result::success(response)).unwrap();
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

impl client::rpc::server::Rpc for GetContent {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: GetContentRequest = try_rpc!(context, serde_json::from_value(args));
        let buffers = self.buffers.read().unwrap();

        let buffer = try_rpc!(context, buffers.get(request.buffer_index));

        let response = GetContentResponse {
            content: buffer.to_string(),
        };
        context.finish(rpc::Result::success(response)).unwrap();
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

impl client::rpc::server::Rpc for Open {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        const FILE_PREFIX: &'static str = "file://";
        let mut request: OpenRequest = try_rpc!(context, serde_json::from_value(args));
        if !request.uri.starts_with(FILE_PREFIX) {
            context.finish(rpc::Result::NotHandled).unwrap();
            return;
        }
        request.uri.drain(..FILE_PREFIX.len());

        let mut file = try_rpc!(context, fs::File::open(path::Path::new(&request.uri)));
        let mut content = String::new();
        try_rpc!(context, file.read_to_string(&mut content));

        let buffer = Buffer::from_string(content);

        let mut buffers = self.buffers.write().unwrap();
        let response = OpenResponse {
            buffer_index: buffers.new_buffer(buffer),
        };
        context.finish(rpc::Result::success(response)).unwrap();
    }
}

// NOCOM(#sirver): add a test for this.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListRequest;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ListResponse {
    pub buffer_indices: Vec<usize>,
}

struct List {
    buffers: Arc<RwLock<BuffersManager>>,
}

impl client::rpc::server::Rpc for List {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let _: ListRequest = try_rpc!(context, serde_json::from_value(args));

        let buffers = self.buffers.read().unwrap();
        let response = ListResponse {
            buffer_indices: buffers.keys().map(|c| *c).collect(),
        };
        context.finish(rpc::Result::success(response)).unwrap();
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
    client: client::ThinClient,
}

impl BuffersManager {
    fn new(client: client::ThinClient) -> Self {
        BuffersManager {
            next_buffer_index: 0,
            buffers: HashMap::new(),
            client: client
        }
    }

    fn new_buffer(&mut self, buffer: Buffer) -> usize {
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

    fn delete_buffer(&mut self, buffer_index: usize) -> Result<(), BufferError> {
        try!(self.buffers.remove(&buffer_index).ok_or(BufferError::UnknownBuffer));

        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.client.call("on.buffer.deleted", &BufferDeleted {
            buffer_index: buffer_index,
        });

        Ok(())
    }

    fn get(&self, index: usize) -> Result<&Buffer, BufferError> {
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



pub struct BufferPlugin {
    client: client::Client,
    buffers: Arc<RwLock<BuffersManager>>,
}

impl BufferPlugin {
    pub fn new(socket_name: &path::Path) -> Self {
        let client = client::Client::connect_unix(socket_name).unwrap();

        let plugin = BufferPlugin {
            buffers: Arc::new(RwLock::new(BuffersManager::new(client.clone()))),
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

        let list = Box::new(List { buffers: plugin.buffers.clone() });
        plugin.client.new_rpc("buffer.list", list);

        plugin
    }
}
