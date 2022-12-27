// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

extern crate serde;
extern crate serde_json;
extern crate swiboe;
extern crate uuid;

use std::fs;
use std::io::Write;
use std::path;
use swiboe::client;
use swiboe::testing;

mod core;
mod plugin_buffer;

pub struct CallbackRpc<F> {
    pub priority: u16,
    pub callback: F,
}

impl<F: Send + Sync> client::rpc::server::Rpc for CallbackRpc<F>
where
    F: Fn(client::rpc::server::Context, serde_json::Value) + Send,
{
    fn call(&self, context: client::rpc::server::Context, args: serde_json::Value) {
        (self.callback)(context, args);
    }
    fn priority(&self) -> u16 {
        self.priority
    }
}

pub fn create_file(t: &testing::TestHarness, name: &str, content: &str) -> path::PathBuf {
    let mut file_name = t.temp_directory.path().to_path_buf();
    file_name.push(name);

    let mut f = fs::File::create(&file_name).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    file_name
}
