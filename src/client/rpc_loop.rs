use ::client::event_loop;
use ::client::rpc;
use mio;
use std::collections::HashMap;
use std::sync::mpsc;


pub struct NewRpc<'a> {
    name: String,
    rpc: Box<rpc::server::Rpc + 'a>,
}

impl<'a> NewRpc<'a> {
    pub fn new(name: String,
           rpc: Box<rpc::server::Rpc + 'a>) -> Self {
        NewRpc {
            name: name,
            rpc: rpc,
        }
    }
}

pub type CommandSender = mpsc::Sender<Command>;
pub enum Command {
    Quit,
    Received(::ipc::Message),
    OutgoingCall(String, ),
    CancelOutgoingRpc(String),
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
    event_loop_sender: mio::Sender<event_loop::Command>,
    running_rpc_calls: HashMap<String, RunningRpc>,
}

impl<'a> RpcLoop<'a> {
    fn spin_forever(&mut self, commands: mpsc::Receiver<Command>, new_rpcs:
                    mpsc::Receiver<NewRpc<'a>>) {
        'outer: loop {
            select! {
                new_rpc = new_rpcs.recv() => match new_rpc {
                    Err(_) => break 'outer,
                    Ok(new_rpc) => {
                        self.remote_procedures.insert(new_rpc.name, new_rpc.rpc);
                    },
                },
                command = commands.recv() => match command {
                    Err(_) => break 'outer,
                    Ok(command) => {
                        if self.handle_command(command) == true {
                            break 'outer;
                        }
                    },
                }
            }
        }
    }

    fn handle_command(&mut self, command: Command) -> bool {
        match command {
            Command::Quit => return true,
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
            Command::Send(message) => {
                self.event_loop_sender.send(message).expect("Command::Send");
            },
        };
        return false;
    }
}

pub fn spawn<'a>(commands: mpsc::Receiver<Command>,
                 new_rpcs: mpsc::Receiver<NewRpc<'a>>,
                 event_loop_sender: mio::Sender<event_loop::Command>) -> ::thread_scoped::JoinGuard<'a, ()>
{
    unsafe {
        ::thread_scoped::scoped(move || {
            let mut thread = RpcLoop {
                remote_procedures: HashMap::new(),
                running_rpc_calls: HashMap::new(),
                event_loop_sender: event_loop_sender,
            };
            thread.spin_forever(commands, new_rpcs);
        })
    }
}
