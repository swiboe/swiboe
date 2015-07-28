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
    // NOCOM(#sirver): rename to internal.
    Local(&'static str),
    Remote(RemotePluginId),
}

// NOCOM(#sirver): Really a struct?
pub struct FunctionCallContext {
    // NOCOM(#sirver): maybe force this to be a uuid?
    pub context: String,
    pub function: String,
    pub args: json::value::Value,
    pub commands: CommandSender,
    pub caller: PluginId,
    // NOCOM(#sirver): needs some sort of backchannel? Needs to know who send it.
}

pub enum FunctionResult {
    DONE,
}

pub trait Plugin: Send {
    fn name(&self) -> &'static str;
    fn id(&self) -> PluginId;
    fn broadcast(&self, data: &json::value::Value);
    fn call(&self, context: FunctionCallContext) -> FunctionResult;
}

pub mod remote;
