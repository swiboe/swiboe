// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use ::client;
use ::error::Result;
use ::rpc;
use serde_json;
use std::convert;
use std::path;

struct Logger;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct LoggerRequest {
    pub level: String,
    pub message: String,
    pub time: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct LoggerResponse;

impl client::rpc::server::Rpc for Logger {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: LoggerRequest = try_rpc!(context, serde_json::from_value(args));
        // NOCOM make output customizable
        println!("{} - [{}] - {}", request.time, request.level, request.message);
        context.finish(rpc::Result::success(LoggerResponse)).unwrap();
    }
}

pub struct LoggerPlugin {
    client: client::Client,
}

impl LoggerPlugin {
    pub fn new(socket_name: &path::Path) -> Result<Self> {
        let client = try!(client::Client::connect_unix(socket_name));
        let mut plugin = LoggerPlugin {
            client: client,
        };
        let logger = Box::new(Logger);
        try!(plugin.client.new_rpc("logger", logger));
        Ok(plugin)
    }
}
