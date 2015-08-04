use serde::json;
use std::path;
use std::sync::{RwLock, Arc};
use std::collections::HashMap;
use super::Result;
use super::client::{self, Client, RemoteProcedure};
use super::ipc;

// NOCOM(#sirver): make a new package rpc and move some stuff itno that?

// NOCOM(#sirver): messages must contain an indication of the type or so.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BufferCreated {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BufferDeleted {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewRequest;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct NewResponse {
    pub buffer_index: usize,
}

struct New {
    client_handle: client::ClientHandle,
    buffers: Arc<RwLock<BuffersManager>>,
}

impl<'a> RemoteProcedure for New {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        // NOCOM(#sirver): need some: on bad request results
        // NOCOM(#sirver): needs some understanding what happens on extra values.
        let request: NewRequest = json::from_value(args).unwrap();
        let mut buffers = self.buffers.write().unwrap();

        let response = NewResponse {
            buffer_index: buffers.create_buffer(),
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
    client_handle: client::ClientHandle,
    buffers: Arc<RwLock<BuffersManager>>,
}

impl<'a> RemoteProcedure for Delete {
    fn call(&mut self, args: json::Value) -> ipc::RpcResult {
        // NOCOM(#sirver): need some: on bad request results
        // NOCOM(#sirver): needs some understanding what happens on extra values.
        let request: DeleteRequest = json::from_value(args).unwrap();
        let mut buffers = self.buffers.write().unwrap();
        // NOCOM(#sirver): handle errors
        buffers.delete_buffer(request.buffer_index).unwrap();

        let response = DeleteResponse;
        ipc::RpcResult::success(response)
    }
}

struct BuffersManager {
    client_handle: client::ClientHandle,
    next_buffer_index: usize,
    buffers: HashMap<usize, String>,
}

impl BuffersManager {
    fn new(client_handle: client::ClientHandle) -> Self {
        BuffersManager {
            client_handle: client_handle,
            next_buffer_index: 0,
            buffers: HashMap::new(),
        }
    }

    fn create_buffer(&mut self) -> usize {
        let current_buffer_index = self.next_buffer_index;
        self.next_buffer_index += 1;

        self.buffers.insert(current_buffer_index, String::new());

        // NOCOM(#sirver): should call a callback instead
        // self.client_handle.call("core.broadcast", &BufferCreated {
            // buffer_index: current_buffer_index,
        // }).wait().unwrap();
        current_buffer_index
    }

    fn delete_buffer(&mut self, buffer_index: usize) -> Result<()> {
        self.buffers.remove(&buffer_index);

        // NOCOM(#sirver): should call a callback instead
        // self.client_handle.call("core.broadcast", &BufferDeleted {
            // buffer_index: buffer_index,
        // }).wait().unwrap();
        Ok(())
    }
}

pub struct BufferPlugin<'a> {
    client: Client<'a>,
    buffers: Arc<RwLock<BuffersManager>>,
}

impl<'a> BufferPlugin<'a> {
    // NOCOM(#sirver): is 'b needed?
    pub fn new(socket_name: &path::Path) -> Self {
        let client = Client::connect(socket_name);

        let plugin = BufferPlugin {
            buffers: Arc::new(RwLock::new(BuffersManager::new(client.client_handle()))),
            client: client,
        };

        let new = Box::new(New {
            client_handle: plugin.client.client_handle(),
            buffers: plugin.buffers.clone(),
        });
        plugin.client.register_function("buffer.new", new);

        let delete = Box::new(Delete {
            client_handle: plugin.client.client_handle(),
            buffers: plugin.buffers.clone(),
        });
        plugin.client.register_function("buffer.delete", delete);
        plugin
    }
}
