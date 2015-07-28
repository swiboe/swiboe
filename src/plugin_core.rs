use super::server::{Function, FunctionResult, CommandSender, Command, FunctionCallContext};
use serde::json;

struct RemoteFunction {
    // NOCOM(#sirver): client?
    name: String,
}

// NOCOM(#sirver): is this async or sync?
impl Function for RemoteFunction {
    fn name(&self) -> &str { &self.name }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        // NOCOM(#sirver): implement this
        FunctionResult::DONE
    }
}

struct CoreExit;
impl Function for CoreExit {
    fn name(&self) -> &str { "core.exit" }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        context.commands.send(Command::Shutdown).unwrap();
        FunctionResult::DONE
    }
}

struct CoreRegisterFunction;
impl Function for CoreRegisterFunction {
    fn name(&self) -> &str { "core.register_function" }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        let function = RemoteFunction {
            name: context.args.find("name").unwrap().as_string().unwrap().into(),
        };
        context.commands.send(Command::RegisterFunction(Box::new(function))).unwrap();
        println!("#sirver args: {:#?}", context.args);
        // NOCOM(#sirver): implement
        // command_sender.send(Command::RegisterFunction(Box::new(function))).unwrap();
        // commands.send(Command::Shutdown).unwrap();
        FunctionResult::DONE
    }
}

struct CoreBroadcast;
impl Function for CoreBroadcast {
    fn name(&self) -> &str { "core.broadcast" }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        context.commands.send(Command::Broadcast(context.args)).unwrap();
        FunctionResult::DONE
    }
}


pub fn register(command_sender: &CommandSender) {
    command_sender.send(Command::RegisterFunction(Box::new(CoreExit))).unwrap();
    command_sender.send(Command::RegisterFunction(Box::new(CoreRegisterFunction))).unwrap();
    command_sender.send(Command::RegisterFunction(Box::new(CoreBroadcast))).unwrap();
}
