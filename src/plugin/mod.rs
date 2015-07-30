use mio;
use serde::json;
use super::server::{CommandSender};

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct RemotePluginId {
    pub serial: u64,
    pub token: mio::Token,
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum PluginId {
    Internal(&'static str),
    Remote(RemotePluginId),
}

// TODO(sirver): Document all the structs.
pub struct FunctionCallContext {
    pub context: String,
    pub function: String,
    pub args: json::value::Value,
    pub commands: CommandSender,
    pub caller: PluginId,
}

pub enum FunctionResult {
    /// The function has handled the call. No other function should try to handle it.
    HANDLED,

    /// This function cannot handle this call. Another function can try.
    NOT_HANDLED,

    /// The function has deferred the call to another plugin which will handle it.
    DEFERRED,
}

pub trait Plugin: Send {
    fn name(&self) -> &'static str;
    fn id(&self) -> PluginId;
    fn broadcast(&self, data: &json::value::Value);
    fn call(&self, context: FunctionCallContext) -> FunctionResult;
}

pub mod remote;
