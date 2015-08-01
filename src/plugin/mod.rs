use mio;
use super::ipc;
use super::server::{CommandSender};

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct RemotePluginId {
    pub serial: u64,
    pub token: mio::Token,
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PluginId {
    Internal(&'static str),
    Remote(RemotePluginId),
}

// TODO(sirver): Document all the structs.
pub struct FunctionCallContext {
    pub rpc_call: ipc::RpcCall,
    pub commands: CommandSender,
    pub caller: PluginId,
}

pub enum FunctionResult {
    /// The function has handled the call. No other function should try to handle it.
    Handled,

    /// This function cannot handle this call. Another function can try.
    NotHandled,

    /// The function has delegated the call to another plugin which will handle it.
    Delegated,
}

pub trait Plugin: Send {
    fn name(&self) -> &'static str;
    fn id(&self) -> PluginId;
    fn send(&self, message: &ipc::Message);
    fn call(&self, context: FunctionCallContext) -> FunctionResult;
}

pub mod remote;
