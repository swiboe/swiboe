use serde::json;
use std::collections::HashMap;
use std::path;
use std::sync::{RwLock, Arc};
use super::Result;
use super::client;
use super::ipc;

// NOCOM(#sirver): make a new package rpc and move some stuff itno that?

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
    buffers: Arc<RwLock<BuffersManager>>,
}

impl client::RemoteProcedure for New {
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
    buffers: Arc<RwLock<BuffersManager>>,
}

impl client::RemoteProcedure for Delete {
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
    next_buffer_index: usize,
    buffers: HashMap<usize, String>,
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

    fn create_buffer(&mut self) -> usize {
        let current_buffer_index = self.next_buffer_index;
        self.next_buffer_index += 1;

        self.buffers.insert(current_buffer_index, String::new());

        // NOCOM(#sirver): new is not good. should be create.
        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.rpc_caller.call("on.buffer.new", &BufferCreated {
            buffer_index: current_buffer_index,
        });
        current_buffer_index
    }

    fn delete_buffer(&mut self, buffer_index: usize) -> Result<()> {
        self.buffers.remove(&buffer_index);

        // Fire the callback, but we do not wait for it's conclusion.
        let _ = self.rpc_caller.call("on.buffer.deleted", &BufferDeleted {
            buffer_index: buffer_index,
        });

        Ok(())
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

        let new = Box::new(New {
            buffers: plugin.buffers.clone(),
        });
        plugin.client.new_rpc("buffer.new", new);

        let delete = Box::new(Delete {
            buffers: plugin.buffers.clone(),
        });
        plugin.client.new_rpc("buffer.delete", delete);
        plugin
    }
}
