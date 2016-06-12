// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::client;
use ::plugin::buffer::base;
use ::rpc;
use serde_json;
use std::convert;
use std::sync::{RwLock, Arc};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Request {
    pub buffer_index: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Response;

pub struct Rpc {
    pub buffers: Arc<RwLock<base::BuffersManager>>,
}

impl client::rpc::server::Rpc for Rpc {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        let request: Request = try_rpc!(context, serde_json::from_value(args));
        let mut buffers = self.buffers.write().expect("Delete::call: locking buffers.");
        try_rpc!(context, buffers.delete_buffer(request.buffer_index));

        let response = Response;
        context.finish(rpc::Result::success(response)).expect("Delete::call: finish");
    }
}
