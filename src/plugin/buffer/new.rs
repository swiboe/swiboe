// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use client;
use plugin::buffer::base;
use rpc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert;
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Request {
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Response {
    pub buffer_index: usize,
}

pub struct Rpc {
    pub buffers: Arc<RwLock<base::BuffersManager>>,
}

// NOCOM(#sirver): what does serde do if there are extra values in the JSON?
impl client::rpc::server::Rpc for Rpc {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        // NOCOM(#sirver): need testing for bad request results
        let request: Request = try_rpc!(context, serde_json::from_value(args));
        let mut buffers = self.buffers.write().unwrap();

        let buffer = match request.content {
            Some(content) => base::Buffer::from_string(content),
            None => base::Buffer::new(),
        };

        let response = Response {
            buffer_index: buffers.new_buffer(buffer),
        };
        context.finish(rpc::Result::success(response)).unwrap();
    }
}
