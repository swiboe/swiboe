use super::server::{Function, FunctionResult, CommandSender, Command};
use serde::json;

struct CoreExit;
impl<'a> Function<'a> for CoreExit {
    fn name(&self) -> &'a str { "core.exit" }

    fn call(&self, _: json::value::Value, commands: &CommandSender<'a>) -> FunctionResult {
        commands.send(Command::Shutdown).unwrap();
        FunctionResult::DONE
    }
}

pub fn register(command_sender: &CommandSender) {
    command_sender.send(Command::RegisterFunction(Box::new(CoreExit))).unwrap();
}
