use mio;
use serde::json;
use super::super::ipc_bridge;
use super::{PluginId, RemotePluginId, Plugin, FunctionCallContext, FunctionResult};

pub struct RemotePlugin {
    pub id: PluginId,
    pub ipc_brigde_comands: mio::Sender<ipc_bridge::Command>,
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

    fn broadcast(&self, data: &json::value::Value) {
        let s = json::to_string(&data).unwrap();
        self.ipc_brigde_comands.send(
            ipc_bridge::Command::SendData(self.remote_id(), s)).unwrap();
    }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        let data = json::builder::ObjectBuilder::new()
                .insert("context".into(), context.context)
                .insert("function".into(), context.function)
                .insert("args".into(), context.args)
                .unwrap();
        let s = json::to_string(&data).unwrap();
        self.ipc_brigde_comands.send(
            ipc_bridge::Command::SendData(self.remote_id(), s)).unwrap();
        FunctionResult::DONE
    }
}
