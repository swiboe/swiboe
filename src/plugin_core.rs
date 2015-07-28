use super::server::{Function, FunctionResult, CommandSender, Command};
use serde::json;

struct CoreExit;
impl<'a, 'b> Function<'a, 'b> for CoreExit {
    fn name(&self) -> &'a str {
        "core.exit"
    }

    fn call(&self, _: json::value::Value, commands: &CommandSender<'a, 'b>) -> FunctionResult {
        commands.send(Command::SHUTDOWN).unwrap();
        FunctionResult::DONE
    }
}

pub fn register(command_sender: &CommandSender) {
    command_sender.send(Command::REGISTER_FUNCTION(Box::new(CoreExit))).unwrap();
}
