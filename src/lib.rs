#![feature(custom_derive, plugin)]
#![feature(drain)]
#![feature(mpsc_select)]
#![feature(result_expect)]
#![plugin(serde_macros)]

// NOCOM(#sirver): move travis configuration into shell scripts like in UltiSnips.

extern crate libc;
extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate thread_scoped;
extern crate threadpool;
extern crate time;
extern crate uuid;

#[macro_export]
macro_rules! try_rpc {
    ($sender:ident, $expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => {
            // TODO(sirver): Not sure if unwrap() here is okay.
            $sender.finish($crate::rpc::Result::Err(convert::From::from(err))).unwrap();
            return;
        }
    })
}

mod ipc;
mod ipc_bridge;
pub mod client;
pub mod error;
pub mod plugin_buffer;
pub mod plugin_core;
pub mod plugin_list_files;
pub mod rpc;
pub mod server;

pub use error::{Error, ErrorKind, Result};
