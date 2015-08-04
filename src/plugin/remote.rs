use mio;
use super::super::ipc;
use super::super::ipc_bridge;
use super::{PluginId, Plugin, FunctionCallContext, FunctionResult};

#[derive(Debug)]
pub struct RemotePlugin {
    pub id: PluginId,
    pub ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
}

impl Plugin for RemotePlugin {
    // NOCOM(#sirver): name does not fit :(
    fn name(&self) -> &'static str { "remote_plugin" }
    fn id(&self) -> PluginId {
        self.id
    }

    fn send(&self, message: &ipc::Message) {
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.id(), message.clone())).unwrap();
    }

    fn call(&self, context: &FunctionCallContext) -> FunctionResult {
        // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
        // able to move again.
        let message = ipc::Message::RpcCall(context.rpc_call.clone());
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.id(), message)).unwrap();
        FunctionResult::Delegated
    }
}
