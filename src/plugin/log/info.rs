// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use ::client;
use ::plugin::log;
use serde_json;

pub type Request = log::base::Request;

pub type Response = log::base::Response;

pub struct Rpc;

impl client::rpc::server::Rpc for Rpc {
    fn call(&self, context: client::rpc::server::Context, args: serde_json::Value) {
        log::base::log(context, "INFO", args)
    }
}
