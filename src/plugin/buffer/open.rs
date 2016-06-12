// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::client;
use ::plugin::buffer::base;
use ::rpc;
use serde_json;
use std::convert;
use std::fs;
use std::path;
use std::io::Read;
use std::sync::{RwLock, Arc};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Request {
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Response {
    pub buffer_index: usize,
}

pub struct Rpc {
    pub buffers: Arc<RwLock<base::BuffersManager>>,
}

impl client::rpc::server::Rpc for Rpc {
    fn call(&self, mut context: client::rpc::server::Context, args: serde_json::Value) {
        const FILE_PREFIX: &'static str = "file://";
        let mut request: Request = try_rpc!(context, serde_json::from_value(args));
        if !request.uri.starts_with(FILE_PREFIX) {
            context.finish(rpc::Result::NotHandled).unwrap();
            return;
        }
        request.uri.drain(..FILE_PREFIX.len());

        let mut file = try_rpc!(context, fs::File::open(path::Path::new(&request.uri)));
        let mut content = String::new();
        try_rpc!(context, file.read_to_string(&mut content));

        let buffer = base::Buffer::from_string(content);

        let mut buffers = self.buffers.write().unwrap();
        let response = Response {
            buffer_index: buffers.new_buffer(buffer),
        };
        context.finish(rpc::Result::success(response)).unwrap();
    }
}

