// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

// NOCOM(#sirver): add a test for this.

use client;
use plugin::buffer::base;
use rpc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert;
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Request;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Response {
    pub buffer_indices: Vec<usize>,
}

pub struct Rpc {
    pub buffers: Arc<RwLock<base::BuffersManager>>,
}

impl client::rpc::server::Rpc for Rpc {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let _: Request = try_rpc!(context, serde_json::from_value(args));

        let buffers = self.buffers.read().unwrap();
        let response = Response {
            buffer_indices: buffers.keys().map(|c| *c).collect(),
        };
        context.finish(rpc::Result::success(response)).unwrap();
    }
}
