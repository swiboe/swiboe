// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use server::Server;
use std::path::PathBuf;
use tempdir::TempDir;

pub struct TestHarness {
    server: Option<Server>,
    pub socket_name: PathBuf,
    pub temp_directory: TempDir,
}

impl TestHarness {
    pub fn new() -> Self {
        let temp_directory = TempDir::new("swiboe").unwrap();

        let mut socket_name = temp_directory.path().to_path_buf();
        socket_name.push("_socket");

        let server = Server::launch(&socket_name, &[]).unwrap();

        TestHarness {
            server: Some(server),
            socket_name: socket_name,
            temp_directory: temp_directory,
        }
    }

    pub fn wait_for_shutdown(&mut self) {
        self.server.as_mut().unwrap().wait_for_shutdown();
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        self.server.take().unwrap().shutdown();
    }
}
