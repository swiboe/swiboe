use mio;
use super::super::ipc;
use super::super::ipc_bridge;
use super::{PluginId, RemotePluginId, Plugin, FunctionCallContext, FunctionResult};

#[derive(Debug)]
pub struct RemotePlugin {
    pub id: PluginId,
    pub ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
}

impl RemotePlugin {
    fn remote_id(&self) -> RemotePluginId {
        if let PluginId::Remote(remote_id) = self.id {
            return remote_id;
        }
        panic!("RemotePlugin with non ::Remote() id.");
    }
}

impl Plugin for RemotePlugin {
    // NOCOM(#sirver): name does not fit :(
    fn name(&self) -> &'static str { "remote_plugin" }
    fn id(&self) -> PluginId {
        self.id
    }

    fn broadcast(&self, message: &ipc::Message) {
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.remote_id(), message.clone())).unwrap();
    }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        let message = ipc::Message::RpcCall(context.rpc_call);
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.remote_id(), message)).unwrap();
        FunctionResult::Delegated
    }
}
