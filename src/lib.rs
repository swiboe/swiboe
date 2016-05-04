// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate libc;
extern crate mio;
extern crate serde;
extern crate serde_json;
extern crate tempdir;
extern crate threadpool;
extern crate time;
extern crate unix_socket;
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
pub mod client;
pub mod error;
pub mod plugin;
pub mod rpc;
pub mod spinner;
pub mod server;
pub mod testing;

pub use error::{Error, Result};
