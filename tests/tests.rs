#![feature(unboxed_closures)]

extern crate serde;
extern crate switchboard;
extern crate tempdir;
extern crate uuid;

use serde::json;
use switchboard::client;
use switchboard::ipc;

mod support;

mod core;
mod plugin_buffer;

pub struct CallbackProcedure<F> {
    pub callback: F,
}

impl<F> client::RemoteProcedure for CallbackProcedure<F> where
        F: Fn(json::Value) -> ipc::RpcResult + Send
{
    fn call(&mut self, args: json::Value) -> ipc::RpcResult { (self.callback)(args) }
}
