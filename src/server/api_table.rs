// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use server::ipc_bridge;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ApiInfo {
    pub client_id: ipc_bridge::ClientId,
    pub priority: u16,
}

pub struct ApiTable {
    // NOCOM should use a better datastruct instead of Vec for get_next()
    name_infos: HashMap<String, Vec<ApiInfo>>,
}

impl ApiTable {
    pub fn new() -> Self {
        ApiTable {
            name_infos: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, info: ApiInfo) {
        let infos = self.name_infos.entry(name).or_insert(Vec::new());
        let index = match infos.binary_search_by(|probe| probe.priority.cmp(&info.priority)) {
            Ok(idx) => idx,
            Err(idx) => idx,
        };
        infos.insert(index, info);
    }

    pub fn deregister_by_client(&mut self, client_id: &ipc_bridge::ClientId) {
        let mut empty_names = Vec::new();
        for (name, infos) in self.name_infos.iter_mut() {
            infos.retain(|info| info.client_id != *client_id);
            if infos.is_empty() {
                empty_names.push(name.clone());
            }
        }
        for name in empty_names {
            self.name_infos.remove(&name);
        }
    }

    pub fn get_first(&self, name: &String) -> Option<&ApiInfo> {
        match self.name_infos.get(name) {
            Some(infos) => infos.first(),
            None => None,
        }
    }

    pub fn get_next(&self, name: &String, client_id: &ipc_bridge::ClientId) -> Option<&ApiInfo> {
        match self.name_infos.get(name) {
            Some(infos) => infos
                .iter()
                .skip_while(|info| info.client_id != *client_id)
                .nth(1),
            None => None,
        }
    }
}
