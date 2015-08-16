#![allow(deprecated)]

use ::plugin_core::NewRpcRequest;
use mio::unix::UnixStream;
use mio;
use serde;
use std::path;
use std::sync::mpsc;
use std::thread;

pub struct Client<'a> {
    event_loop_commands: mio::Sender<event_loop::Command>,
    rpc_loop_commands: mpsc::Sender<rpc_loop::Command<'a>>,

    _rpc_loop_thread_join_guard: thread::JoinGuard<'a, ()>,
    _event_loop_thread_join_guard: thread::JoinGuard<'a, ()>,
}

impl<'a> Client<'a> {
    pub fn connect(socket_name: &path::Path) -> Self {
        let stream = UnixStream::connect(&socket_name).unwrap();

        let (commands_tx, commands_rx) = mpsc::channel();
        let (event_loop_thread, event_loop_commands) = event_loop::spawn(stream, commands_tx.clone());

        Client {
            event_loop_commands: event_loop_commands.clone(),
            rpc_loop_commands: commands_tx,
            _rpc_loop_thread_join_guard: rpc_loop::spawn(commands_rx, event_loop_commands),
            _event_loop_thread_join_guard: event_loop_thread,
        }
    }

    pub fn new_rpc(&self, name: &str, rpc: Box<rpc::server::Rpc + 'a>) {
        // NOCOM(#sirver): what happens when this is already inserted? crash probably
        let mut rpc = self.call("core.new_rpc", &NewRpcRequest {
            priority: rpc.priority(),
            name: name.into(),
        });
        let success = rpc.wait().unwrap();
        // NOCOM(#sirver): report failure.

        self.rpc_loop_commands.send(rpc_loop::Command::NewRpc(name.into(), rpc)).expect("NewRpc");
    }

    pub fn call<T: serde::Serialize>(&self, function: &str, args: &T) -> rpc::client::Context {
        rpc::client::Context::new(&self.event_loop_commands, function, args).unwrap()
    }

    pub fn new_sender(&self) -> Sender {
        Sender {
            event_loop_commands: self.event_loop_commands.clone(),
        }
    }
}

impl<'a> Drop for Client<'a> {
    fn drop(&mut self) {
        // Either thread might have panicked at this point, so we can not rely on the sends to go
        // through. We just tell both (again) to Quit and hope they actually join.
        let _ = self.rpc_loop_commands.send(rpc_loop::Command::Quit);
        let _ = self.event_loop_commands.send(event_loop::Command::Quit);
    }
}

#[derive(Clone)]
pub struct Sender {
    event_loop_commands: mio::Sender<event_loop::Command>,
}

// NOCOM(#sirver): figure out the difference between a Sender, an Context and come up with better
// names.
impl Sender {
    pub fn call<T: serde::Serialize>(&self, function: &str, args: &T) -> rpc::client::Context {
        rpc::client::Context::new(&self.event_loop_commands, function, args).unwrap()
    }
}

mod event_loop;
mod rpc_loop;

pub mod rpc;
