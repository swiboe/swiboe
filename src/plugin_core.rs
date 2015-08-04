use serde::json;
use super::ipc;
use super::plugin;
use super::server;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterFunctionArgs {
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

    pub fn call(&self, context: &plugin::FunctionCallContext) -> ipc::RpcResult {
        match &context.rpc_call.function as &str {
            "core.exit" => {
                self.commands.send(server::Command::Shutdown).unwrap();
                ipc::RpcResult::success(())
            },
            "core.broadcast" => {
                self.commands.send(server::Command::Broadcast(
                        ipc::Message::Broadcast(context.rpc_call.args.clone()))).unwrap();
                ipc::RpcResult::success(())
            },
            // NOCOM(#sirver): These args can be pulled out into Serializable structs.
            "core.register_function" => {
                let args: RegisterFunctionArgs = match json::from_value(context.rpc_call.args.clone()) {
                    Ok(args) => args,
                    // NOCOM(#sirver): report errors somehow?
                    Err(_) => panic!("Invalid arguments"),
                };

                self.commands.send(
                    server::Command::RegisterFunction(context.caller, args.name, args.priority)).unwrap();
                ipc::RpcResult::success(())
            },
            _ => panic!("{} was called, but is not a core function.", context.rpc_call.function),
        }
    }
}

// NOCOM(#sirver): kill broadcast
