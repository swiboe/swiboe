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

#[derive(Debug)]
pub struct Plugin {
    pub client_id: ipc_bridge::ClientId,
    pub ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
}

impl Plugin {
    // NOCOM(#sirver): is that used?
    pub fn client_id(&self) -> ipc_bridge::ClientId {
        self.client_id
    }

    pub fn send(&self, message: &ipc::Message) {
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.client_id(), message.clone())).unwrap();
    }

    pub fn call(&self, context: &FunctionCallContext) {
        // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
        // able to move again.
        let message = ipc::Message::RpcCall(context.rpc_call.clone());
        self.ipc_bridge_commands.send(
            ipc_bridge::Command::SendData(self.client_id(), message)).unwrap();
    }
}
