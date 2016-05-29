// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::error::Result;
use ::plugin;
use mio;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::thread;

// NOCOM(#sirver): when a client disconnects and we still try to call one of it's rpcs, we never
// get an error back - this will effectively interrupt the rpc call stack.
// NOCOM(#sirver): document everything.

pub struct Server {
    unix_domain_socket_name: PathBuf,
    tcp_addresses: Vec<String>,
    commands: swiboe::SenderTo,
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
    swiboe_thread: Option<thread::JoinHandle<()>>,
    event_loop_thread: Option<thread::JoinHandle<()>>,
    buffer_plugin: Option<plugin::buffer::BufferPlugin>,
    list_files_plugin: Option<plugin::list_files::ListFilesPlugin>,
    log_plugin: Option<plugin::log::Plugin>,
}

impl Server {
    pub fn launch(unix_domain_socket_name: &Path, tcp_addresses: &[&str]) -> Result<Self> {
        let (tx, rx) = channel();

        let mut event_loop = mio::EventLoop::new().expect("Could not create an event loop.");

        let mut server = Server {
            unix_domain_socket_name: unix_domain_socket_name.to_path_buf(),
            tcp_addresses: tcp_addresses.iter().map(|slice| slice.to_string()).collect(),
            commands: tx.clone(),
            ipc_bridge_commands: event_loop.channel(),
            buffer_plugin: None,
            list_files_plugin: None,
            log_plugin: None,
            swiboe_thread: None,
            event_loop_thread: None,
        };

        server.swiboe_thread = Some(swiboe::spawn(event_loop.channel(), tx.clone(), rx));

        let mut ipc_bridge = ipc_bridge::IpcBridge::new(
            &mut event_loop, &server.unix_domain_socket_name, &server.tcp_addresses,
            server.commands.clone());

        server.event_loop_thread = Some(thread::spawn(move || {
            event_loop.run(&mut ipc_bridge).expect("Could not start event_loop.");
        }));

        server.buffer_plugin = Some(
            try!(plugin::buffer::BufferPlugin::new(&server.unix_domain_socket_name)));
        server.list_files_plugin = Some(
            try!(plugin::list_files::ListFilesPlugin::new(&server.unix_domain_socket_name)));
        server.log_plugin = Some(
            try!(plugin::log::Plugin::new(&server.unix_domain_socket_name)));
        Ok(server)
    }

    pub fn shutdown(&mut self) {
        // Any of the threads might have already panicked. So we ignore send errors.
        let _ = self.ipc_bridge_commands.send(ipc_bridge::Command::Quit);
        self.wait_for_event_loop_thread_to_shut_down();
        let _ = self.commands.send(swiboe::Command::Quit);
        self.wait_for_swiboe_thread_to_shut_down();

        self.wait_for_shutdown();
    }

    fn wait_for_event_loop_thread_to_shut_down(&mut self) {
        if let Some(thread) = self.event_loop_thread.take() {
            thread.join().expect("Could not join event_loop_thread.");
        }
    }

    fn wait_for_swiboe_thread_to_shut_down(&mut self) {
        if let Some(thread) = self.swiboe_thread.take() {
            thread.join().expect("Could not join swiboe_thread.");
        }
    }

    pub fn wait_for_shutdown(&mut self) {
        self.wait_for_event_loop_thread_to_shut_down();
        self.wait_for_swiboe_thread_to_shut_down();

        fs::remove_file(&self.unix_domain_socket_name).expect(
            &format!("Could not remove socket {:?}", self.unix_domain_socket_name));
    }
}

mod ipc_bridge;
mod swiboe;
pub mod plugin_core; // NOCOM being a private mod
