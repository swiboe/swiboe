#![allow(deprecated)]

use ::plugin_core::NewRpcRequest;
use mio::unix::UnixStream;
use mio;
use serde;
use std::collections::HashMap;
use std::path;
use std::sync::mpsc;
use std::thread;

pub enum FunctionThreadCommand<'a> {
    Quit,
    NewRpc(String, Box<rpc::server::Rpc + 'a>),
    Call(::rpc::Call),
    Cancel(::rpc::Cancel),
}

struct RunningRpc {
    commands: mpsc::Sender<rpc::server::Command>,
}

impl RunningRpc {
    fn new(commands: mpsc::Sender<rpc::server::Command>) -> Self {
        RunningRpc {
            commands: commands,
        }
    }
}

struct FunctionThread<'a> {
    remote_procedures: HashMap<String, Box<rpc::server::Rpc + 'a>>,
    commands: mpsc::Receiver<FunctionThreadCommand<'a>>,
    event_loop_sender: mio::Sender<event_loop::Command>,
    running_rpc_calls: HashMap<String, RunningRpc>,
}

impl<'a> FunctionThread<'a> {
    fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                FunctionThreadCommand::Quit => break,
                FunctionThreadCommand::NewRpc(name, remote_procedures) => {
                    self.remote_procedures.insert(name, remote_procedures);
                },
                FunctionThreadCommand::Call(rpc_call) => {
                    if let Some(function) = self.remote_procedures.get_mut(&rpc_call.function) {
                        let (tx, rx) = mpsc::channel();
                        function.call(rpc::server::Context::new(
                            rpc_call.context.clone(), rx, self.event_loop_sender.clone()), rpc_call.args);
                        self.running_rpc_calls.insert(rpc_call.context.clone(), RunningRpc::new(tx));
                    }
                    // NOCOM(#sirver): return an error - though if that has happened the
                    // server messed up too.
                }
                FunctionThreadCommand::Cancel(rpc_cancel) => {
                    println!("#sirver rpc_cancel: {:#?}", rpc_cancel);
                    // NOCOM(#sirver): on drop, the rpcservercontext must delete the entry.
                    if let Some(function) = self.running_rpc_calls.remove(&rpc_cancel.context) {
                        // The function might be dead already, so we ignore errors.
                        let _ = function.commands.send(rpc::server::Command::Cancel);
                    }
                }
            }
        }
    }
}

pub struct Client<'a> {
    event_loop_sender: mio::Sender<event_loop::Command>,
    _function_thread_join_guard: thread::JoinGuard<'a, ()>,

    function_thread_sender: mpsc::Sender<FunctionThreadCommand<'a>>,
}

impl<'a> Client<'a> {
    pub fn connect(socket_name: &path::Path) -> Self {
        let stream = UnixStream::connect(&socket_name).unwrap();

        let (commands_tx, commands_rx) = mpsc::channel();
        let (event_loop_thread, event_loop_sender) = event_loop::spawn(stream, commands_tx.clone());

        Client {
            event_loop_sender: event_loop_sender.clone(),
            function_thread_sender: commands_tx,
            _function_thread_join_guard: thread::scoped(move || {
                let mut thread = FunctionThread {
                    remote_procedures: HashMap::new(),
                    running_rpc_calls: HashMap::new(),
                    commands: commands_rx,
                    event_loop_sender: event_loop_sender,
                };
                thread.spin_forever();

                // If the remote side has disconnected us, the event_loop will already be destroyed and
                // this send will fail.
                let _ = thread.event_loop_sender.send(event_loop::Command::Quit);
                event_loop_thread.join();
            }),
        }
    }

    pub fn new_rpc(&self, name: &str, remote_procedure: Box<rpc::server::Rpc + 'a>) {
        // NOCOM(#sirver): what happens when this is already inserted? crash probably
        let mut rpc = self.call("core.new_rpc", &NewRpcRequest {
            priority: remote_procedure.priority(),
            name: name.into(),
        });
        let success = rpc.wait().unwrap();
        // NOCOM(#sirver): report failure.

        self.function_thread_sender.send(FunctionThreadCommand::NewRpc(
                name.into(), remote_procedure)).expect("NewRpc");
    }

    pub fn call<T: serde::Serialize>(&self, function: &str, args: &T) -> rpc::client::Context {
        rpc::client::Context::new(&self.event_loop_sender, function, args).unwrap()
    }

    pub fn new_sender(&self) -> Sender {
        Sender {
            event_loop_sender: self.event_loop_sender.clone(),
        }
    }
}

impl<'a> Drop for Client<'a> {
    fn drop(&mut self) {
        // Either thread might have panicked at this point, so we can not rely on the sends to go
        // through. We just tell both (again) to Quit and hope they actually join.
        let _ = self.function_thread_sender.send(FunctionThreadCommand::Quit);
        let _ = self.event_loop_sender.send(event_loop::Command::Quit);
    }
}

#[derive(Clone)]
pub struct Sender {
    event_loop_sender: mio::Sender<event_loop::Command>,
}

// NOCOM(#sirver): figure out the difference between a Sender, an Context and come up with better
// names.
impl Sender {
    pub fn call<T: serde::Serialize>(&self, function: &str, args: &T) -> rpc::client::Context {
        rpc::client::Context::new(&self.event_loop_sender, function, args).unwrap()
    }
}

mod event_loop;

pub mod rpc;
