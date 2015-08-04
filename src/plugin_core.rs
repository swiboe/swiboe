use serde::json;
use super::ipc;
use super::ipc_bridge;
use super::server;

#[derive(Serialize, Deserialize, Debug)]
pub struct NewRpcRequest {
    pub priority: u16,
    pub name: String,
}

pub struct CorePlugin {
    commands: server::CommandSender,
}

impl CorePlugin {
    pub fn new(commands: server::CommandSender) -> Self {
        CorePlugin {
            commands: commands,
        }
    }

    pub fn call(&self, caller: ipc_bridge::ClientId, rpc_call: &ipc::RpcCall) -> ipc::RpcResult {
        match &rpc_call.function as &str {
            "core.exit" => {
                self.commands.send(server::Command::Shutdown).unwrap();
                ipc::RpcResult::success(())
            },
            // NOCOM(#sirver): These args can be pulled out into Serializable structs.
            "core.new_rpc" => {
                let args: NewRpcRequest = match json::from_value(rpc_call.args.clone()) {
                    Ok(args) => args,
                    // NOCOM(#sirver): report errors somehow?
                    Err(_) => panic!("Invalid arguments"),
                };

                self.commands.send(
                    server::Command::NewRpc(caller, args.name, args.priority)).unwrap();
                ipc::RpcResult::success(())
            },
            // NOCOM(#sirver): this should not panic, but return an error.
            _ => panic!("{} was called, but is not a core function.", rpc_call.function),
        }
    }
}
