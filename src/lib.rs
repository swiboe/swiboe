#![feature(slice_position_elem)]

extern crate mio;
extern crate serde;

// NOCOM(#sirver): what needs to be pub here?
mod buffer;
pub mod server;
pub mod client;
pub mod ipc;
pub mod plugin_core;
