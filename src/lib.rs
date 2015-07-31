#![feature(custom_derive, plugin)]
#![feature(result_expect)]
#![feature(scoped)]
#![plugin(serde_macros)]

extern crate mio;
extern crate serde;
extern crate uuid;

mod ipc_bridge;
mod plugin;
pub mod client;
pub mod ipc;
pub mod plugin_core;
pub mod server;
pub mod error;


pub use error::{Error, ErrorKind, Result};
