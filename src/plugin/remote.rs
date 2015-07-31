use mio;
use serde::json;
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
        // NOCOM(#sirver): context could contain this already. less copy.
        let message = ipc::Message::RpcCall {
            context: context.context,
            function: context.function,
            args: context.args,
        };
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.remote_id(), message)).unwrap();
        FunctionResult::Delegated
    }
}
