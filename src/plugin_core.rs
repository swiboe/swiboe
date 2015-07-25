use super::server::{SupremeServer, Function, FunctionResult};
use serde::json;

// TODO(sirver): move function name into trait?
struct CoreExit;
impl<'a> Function<'a> for CoreExit {
    fn name(&self) -> &'a str {
        "core.exit"
    }

    fn call(&self, args: &json::value::Value) -> FunctionResult {
        println!("#sirver Should shutdown :(");
        // server.shutdown();
        FunctionResult::DONE
    }
}

pub fn register(server: &mut SupremeServer) {
    server.register_function(0, Box::new(CoreExit));
}
