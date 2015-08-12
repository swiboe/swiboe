#![feature(unboxed_closures)]

extern crate serde;
extern crate switchboard;
extern crate tempdir;
extern crate uuid;

use serde::json;
use std::fs;
use std::io::Write;
use std::path;
use switchboard::client;
use switchboard::ipc;

mod support;

mod core;
mod plugin_buffer;

pub struct CallbackProcedure<F> {
    pub priority: u16,
    pub callback: F,
}

impl<F> client::RemoteProcedure for CallbackProcedure<F> where
        F: Fn(json::Value) -> ipc::RpcResult + Send
{
    fn call(&mut self, _: client::RpcSender, args: json::Value) -> ipc::RpcResult {
        (self.callback)(args) }
    fn priority(&self) -> u16 { self.priority }
}

pub fn create_file(t: &support::TestHarness, name: &str, content: &str) -> path::PathBuf {
    let mut file_name = t.temp_directory.path().to_path_buf();
    file_name.push(name);

    let mut f = fs::File::create(&file_name).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    file_name
}
