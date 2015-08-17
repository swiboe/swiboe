use ::ipc_bridge;
use ::rpc;
use ::server;
use serde_json;

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

    pub fn call(&self, caller: ipc_bridge::ClientId, rpc_call: &rpc::Call) -> rpc::Result {
        match &rpc_call.function as &str {
            "core.exit" => {
                self.commands.send(server::Command::Quit).unwrap();
                rpc::Result::success(())
            },
            // NOCOM(#sirver): These args can be pulled out into Serializable structs.
            "core.new_rpc" => {
                let args: NewRpcRequest = match serde_json::from_value(rpc_call.args.clone()) {
                    Ok(args) => args,
                    // NOCOM(#sirver): report errors somehow?
                    Err(_) => panic!("Invalid arguments"),
                };

                self.commands.send(
                    server::Command::NewRpc(caller, args.name, args.priority)).unwrap();
                rpc::Result::success(())
            },
            // NOCOM(#sirver): this should not panic, but return an error.
            _ => panic!("{} was called, but is not a core function.", rpc_call.function),
        }
    }
}
