// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use ::client;
use ::error::Result;
use ::plugin;
use time;


pub struct Plugin {
    _client: client::Client,
}

impl Plugin {
    pub fn new(mut client: client::Client) -> Result<Self> {
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

pub fn current() -> String {
    format!("{}", time::now_utc().rfc3339())
}

mod base;
pub mod debug;
pub mod info;
pub mod warn;
pub mod error;
