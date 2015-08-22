use ::client::event_loop;
use ::client::rpc;
use mio;
use std::collections::HashMap;
use std::sync::mpsc;

pub enum Command<'a> {
    Quit,
    NewRpc(String, Box<rpc::server::Rpc + 'a>),
    Received(::ipc::Message),
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

struct RpcLoop<'a> {
    remote_procedures: HashMap<String, Box<rpc::server::Rpc + 'a>>,
    commands: mpsc::Receiver<Command<'a>>,
    event_loop_sender: mio::Sender<event_loop::Command>,
    running_rpc_calls: HashMap<String, RunningRpc>,
}

impl<'a> RpcLoop<'a> {
    fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                Command::Quit => break,
                Command::NewRpc(name, remote_procedures) => {
                    self.remote_procedures.insert(name, remote_procedures);
                },
                Command::Received(message) => {
                    match message {
                        ::ipc::Message::RpcCall(rpc_call) => {
                            if let Some(function) = self.remote_procedures.get_mut(&rpc_call.function) {
                                let (tx, rx) = mpsc::channel();
                                function.call(rpc::server::Context::new(
                                        rpc_call.context.clone(), rx, self.event_loop_sender.clone()), rpc_call.args);
                                self.running_rpc_calls.insert(rpc_call.context.clone(), RunningRpc::new(tx));
                            }
                            // NOCOM(#sirver): return an error - though if that has happened the
                            // server messed up too.
                        },
                        ::ipc::Message::RpcCancel(rpc_cancel) => {
                            // NOCOM(#sirver): on drop, the rpcservercontext must delete the entry.
                            if let Some(function) = self.running_rpc_calls.remove(&rpc_cancel.context) {
                                // The function might be dead already, so we ignore errors.
                                let _ = function.commands.send(rpc::server::Command::Cancel);
                            }
                        },
                        // NOCOM(#sirver): todo
                        _ => unimplemented!(),
                    }
                },
            }
        }
    }
}

pub fn spawn<'a>(commands_rx: mpsc::Receiver<Command<'a>>,
                 event_loop_sender: mio::Sender<event_loop::Command>) -> ::thread_scoped::JoinGuard<'a, ()>
{
    unsafe {
        ::thread_scoped::scoped(move || {
            let mut thread = RpcLoop {
                remote_procedures: HashMap::new(),
                running_rpc_calls: HashMap::new(),
                commands: commands_rx,
                event_loop_sender: event_loop_sender,
            };
            thread.spin_forever();
        })
    }
}
