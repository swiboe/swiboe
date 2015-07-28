use super::server::{Function, FunctionResult, CommandSender, Command};
use serde::json;

struct RemoteFunction {
    // NOCOM(#sirver): client?
    name: String,
}

impl<'a> Function<'a> for RemoteFunction {
    fn name(&self) -> &str { &self.name }

    fn call(&self, _: json::value::Value, _: &CommandSender<'a>) -> FunctionResult {
        println!("#sirver self.name: {:#?}", self.name);
        FunctionResult::DONE
    }
}

struct CoreExit;
impl<'a> Function<'a> for CoreExit {
    fn name(&self) -> &'a str { "core.exit" }

    fn call(&self, _: json::value::Value, commands: &CommandSender<'a>) -> FunctionResult {
        commands.send(Command::Shutdown).unwrap();
        FunctionResult::DONE
    }
}

struct CoreRegisterFunction;
impl<'a> Function<'a> for CoreRegisterFunction {
    fn name(&self) -> &'a str { "core.register_function" }

    fn call(&self, args: json::value::Value, commands: &CommandSender<'a>) -> FunctionResult {
        let function = RemoteFunction {
            name: args.find("name").unwrap().as_string().unwrap().into(),
        };
        commands.send(Command::RegisterFunction(Box::new(function))).unwrap();
        println!("#sirver args: {:#?}", args);
        // NOCOM(#sirver): implement
        // command_sender.send(Command::RegisterFunction(Box::new(function))).unwrap();
        // commands.send(Command::Shutdown).unwrap();
        FunctionResult::DONE
    }
}

pub fn register(command_sender: &CommandSender) {
    command_sender.send(Command::RegisterFunction(Box::new(CoreExit))).unwrap();
    command_sender.send(Command::RegisterFunction(Box::new(CoreRegisterFunction))).unwrap();
}
