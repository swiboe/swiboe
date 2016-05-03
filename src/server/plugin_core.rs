// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::rpc;
use ::server::ipc_bridge;
use ::server::swiboe;
use serde_json;

#[derive(Serialize, Deserialize, Debug)]
pub struct NewRpcRequest {
    pub priority: u16,
    pub name: String,
}

pub struct CorePlugin {
    commands: swiboe::SenderTo,
}

impl CorePlugin {
    pub fn new(commands: swiboe::SenderTo) -> Self {
        CorePlugin {
            commands: commands,
        }
    }

    pub fn call(&self, caller: ipc_bridge::ClientId, rpc_call: &rpc::Call) -> rpc::Result {
        match &rpc_call.function as &str {
            "core.exit" => {
                self.commands.send(swiboe::Command::Quit).unwrap();
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
                    swiboe::Command::NewRpc(caller, args.name, args.priority)).unwrap();
                rpc::Result::success(())
            },
            // NOCOM(#sirver): this should not panic, but return an error.
            _ => panic!("{} was called, but is not a core function.", rpc_call.function),
        }
    }
}
