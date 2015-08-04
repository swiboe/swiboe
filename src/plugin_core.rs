use serde::json;
use super::ipc;
use super::plugin::{FunctionResult, Plugin, FunctionCallContext, PluginId};
use super::server::{CommandSender, Command};

struct CorePlugin;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterFunctionArgs {
    pub priority: u16,
    pub name: String,
}

impl Plugin for CorePlugin {
    fn name(&self) -> &'static str { "core" }
    // NOCOM(#sirver): rethink this name.
    fn send(&self, _: &ipc::Message) {
    }
    fn id(&self) -> PluginId {
        PluginId::Internal("core")
    }
    fn call(&self, context: &FunctionCallContext) -> FunctionResult {
        match &context.rpc_call.function as &str {
            "core.exit" => {
                context.commands.send(Command::Shutdown).unwrap();
                FunctionResult::Handled
            },
            "core.broadcast" => {
                context.commands.send(Command::Broadcast(ipc::Message::Broadcast(context.rpc_call.args.clone()))).unwrap();
                FunctionResult::Handled
            },
            // NOCOM(#sirver): These args can be pulled out into Serializable structs.
            "core.register_function" => {
                let args: RegisterFunctionArgs = match json::from_value(context.rpc_call.args.clone()) {
                    Ok(args) => args,
                    // NOCOM(#sirver): report errors somehow?
                    Err(_) => panic!("Invalid arguments"),
                };

                context.commands.send(
                    Command::RegisterFunction(context.caller, args.name, args.priority)).unwrap();
                FunctionResult::Handled
            },
            _ => panic!("{} was called, but is not a core function.", context.rpc_call.function),
        }
    }
}

// NOCOM(#sirver): this should use client.
pub fn register(command_sender: &CommandSender) {
    let core = CorePlugin;
    let id = core.id();

    command_sender.send(Command::PluginConnected(Box::new(CorePlugin))).unwrap();

    // NOCOM(#sirver): ugly repetition.
    command_sender.send(Command::RegisterFunction(id, "core.exit".into(), 0)).unwrap();
    // NOCOM(#sirver): kill broadcast
    command_sender.send(Command::RegisterFunction(id, "core.broadcast".into(), 0)).unwrap();
    command_sender.send(Command::RegisterFunction(id, "core.register_function".into(), 0)).unwrap();
}
