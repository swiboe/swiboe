// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use ::client;
use ::rpc;
use std::convert;
use serde_json;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Request {
    pub message: String,
    pub time: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Response;

pub fn log(mut context: client::rpc::server::Context, level: &str, args: serde_json::Value) {
    let request: Request = try_rpc!(context, serde_json::from_value(args));
    // NOCOM make output customizable
    println!("{} - [{}] - {}", request.time, level, request.message);
    context.finish(rpc::Result::success(Response)).unwrap();
}
