// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use client;
use error::Result;
use plugin;
use std::sync::{Arc, RwLock};

pub struct Plugin {
    _client: client::Client,
    _buffers: Arc<RwLock<base::BuffersManager>>,
}

impl Plugin {
    pub fn new(mut client: client::Client) -> Result<Self> {
        let buffers = Arc::new(RwLock::new(base::BuffersManager::new(client.clone()?)));
        let rpc_map = rpc_map! {
            "buffer.new" => new::Rpc { buffers: buffers.clone() },
            "buffer.delete" => delete::Rpc { buffers: buffers.clone() },
            "buffer.get_content" => get_content::Rpc { buffers: buffers.clone() },
            "buffer.open" => open::Rpc { buffers: buffers.clone() },
            "buffer.list" => list::Rpc { buffers: buffers.clone() },
        };
        plugin::register_rpc(&mut client, rpc_map)?;
        Ok(Plugin {
            _client: client,
            _buffers: buffers,
        })
    }
}

mod base;
pub mod delete;
pub mod get_content;
pub mod list;
pub mod new;
pub mod open;
