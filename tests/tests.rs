#![feature(unboxed_closures)]

extern crate serde;
extern crate switchboard;
extern crate tempdir;
extern crate uuid;

mod support;

use serde::json;
use switchboard::client;
use switchboard::ipc;

struct CallbackProcedure<F> {
    callback: F,
}

impl<F> client::RemoteProcedure for CallbackProcedure<F> where
        F: Fn(json::Value) -> ipc::RpcResult + Send
{
    fn call(&mut self, args: json::Value) -> ipc::RpcResult { (self.callback)(args) }
}

mod core;
mod plugin_buffer;
