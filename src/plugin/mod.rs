use mio;
use super::ipc;
use super::ipc_bridge;

// TODO(sirver): Document all the structs.
pub struct FunctionCallContext {
    pub rpc_call: ipc::RpcCall,
    // NOCOM(#sirver): caller is only used as a means to reply with an answer. Pull out an
    // interface?
    pub caller: ipc_bridge::ClientId,
}

pub trait Plugin: Send {
    fn client_id(&self) -> ipc_bridge::ClientId;
    fn send(&self, message: &ipc::Message);
    fn call(&self, context: &FunctionCallContext);
}

pub mod remote;
