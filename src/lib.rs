#![feature(custom_derive, plugin)]
#![feature(drain)]
#![feature(result_expect)]
#![feature(scoped)]
#![plugin(serde_macros)]

// NOCOM(#sirver): move travis configuration into shell scripts like in UltiSnips.

extern crate libc;
extern crate mio;
extern crate serde;
extern crate time;
extern crate uuid;

#[macro_export]
macro_rules! try_rpc {
    ($sender:ident, $expr:expr) => (match $expr {
        Ok(val) => val,
        Err(err) => {
            $sender.finish($crate::rpc::Result::Err(convert::From::from(err)));
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
