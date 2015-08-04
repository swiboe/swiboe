use mio;
use super::super::ipc;
use super::super::ipc_bridge;
use super::{Plugin, FunctionCallContext};

#[derive(Debug)]
pub struct RemotePlugin {
    pub client_id: ipc_bridge::ClientId,
    pub ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
}

// NOCOM(#sirver): no need for that.
impl Plugin for RemotePlugin {
    // NOCOM(#sirver): name does not fit :(
    fn client_id(&self) -> ipc_bridge::ClientId {
        self.client_id
    }

    fn send(&self, message: &ipc::Message) {
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.client_id(), message.clone())).unwrap();
    }

    fn call(&self, context: &FunctionCallContext) {
        // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
        // able to move again.
        let message = ipc::Message::RpcCall(context.rpc_call.clone());
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.client_id(), message)).unwrap();
    }
}
