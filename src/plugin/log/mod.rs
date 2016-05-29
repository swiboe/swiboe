// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use ::client;
use ::error::Result;
use ::plugin;
use std::path;


pub struct Plugin {
    _client: client::Client,
}

impl Plugin {
    pub fn new(socket_name: &path::Path) -> Result<Self> {
        let mut client = try!(client::Client::connect_unix(socket_name));
        try!(plugin::register_rpc(&mut client, rpc_map! {
            "log.debug" => debug::Rpc,
            "log.info" => info::Rpc,
            "log.warn" => warn::Rpc,
            "log.error" => error::Rpc,
        }));
        Ok(Plugin{
            _client: client,
        })
    }
}

mod base;
pub mod debug;
pub mod info;
pub mod warn;
pub mod error;
