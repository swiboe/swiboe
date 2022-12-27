// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.
use client;
use error::Result;
use std::collections::HashMap;

pub type RpcMap = HashMap<String, Box<dyn client::rpc::server::Rpc>>;

macro_rules! rpc_map {
    ($($s:expr => $rpc:expr),*) => {{
        let mut map = $crate::plugin::RpcMap::new();
        $(map.insert(String::from($s), Box::new($rpc));)*
        map
    }};
    ($($s:expr => $rpc:expr),*,) => {
        rpc_map!($($s => $rpc), *)
    };
}

pub fn register_rpc(client: &mut client::Client, map: RpcMap) -> Result<()> {
    for (name, rpc) in map {
        client.new_rpc(&name, rpc)?;
    }
    Ok(())
}

pub mod buffer;
pub mod list_files;
pub mod log;
