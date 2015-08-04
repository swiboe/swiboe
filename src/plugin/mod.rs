use mio;
use super::ipc;

// NOCOM(#sirver): this is really a client id
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct PluginId {
    pub serial: u64,
    pub token: mio::Token,
}

// TODO(sirver): Document all the structs.
pub struct FunctionCallContext {
    pub rpc_call: ipc::RpcCall,
    // NOCOM(#sirver): caller is only used as a means to reply with an answer. Pull out an
    // interface?
    pub caller: PluginId,
}

pub enum FunctionResult {
    /// The function has delegated the call to another plugin which will handle it.
    Delegated,
}

pub trait Plugin: Send {
    fn name(&self) -> &'static str;
    fn id(&self) -> PluginId;
    fn send(&self, message: &ipc::Message);
    fn call(&self, context: &FunctionCallContext) -> FunctionResult;
}

pub mod remote;
