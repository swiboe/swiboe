use ::client::event_loop;
use ::client::rpc;
use ::ipc;
use mio;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;


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
    OutgoingCall(String, mpsc::Sender<::rpc::Response>, ipc::Message),
    CancelOutgoingRpc(String),
    Send(::ipc::Message),
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

// NOCOM(#sirver): name is no longer fitting
struct RpcLoop<'a> {
    remote_procedures: HashMap<String, Arc<Box<rpc::server::Rpc + 'a>>>,
    event_loop_sender: mio::Sender<event_loop::Command>,
    running_rpc_calls: HashMap<String, RunningRpc>,
    command_sender: CommandSender,
    // NOCOM(#sirver): maybe not use a channel to send data to rpcs?
    running_function_calls: HashMap<String, mpsc::Sender<::rpc::Response>>,
}

impl<'a> RpcLoop<'a> {
    fn spin_forever(&mut self, commands: mpsc::Receiver<Command>, new_rpcs:
                    mpsc::Receiver<NewRpc<'a>>) {
        'outer: loop {
            select! {
                new_rpc = new_rpcs.recv() => match new_rpc {
                    Err(_) => break 'outer,
                    Ok(new_rpc) => {
                        self.remote_procedures.insert(new_rpc.name, Arc::new(new_rpc.rpc));
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
                            let function = *function;
                            let (tx, rx) = mpsc::channel();
                            self.running_rpc_calls.insert(rpc_call.context.clone(), RunningRpc::new(tx));
                            let command_sender = self.command_sender.clone();
                            // NOCOM(#sirver): this runs all commands synchronisly on the same
                            // thread that does all work. not great.
                            function.call(rpc::server::Context::new(
                                    rpc_call.context, rx, self.command_sender), rpc_call.args);
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
                    ipc::Message::RpcResponse(rpc_data) => {
                        // NOCOM(#sirver): if this is a streaming RPC, we should cancel the
                        // RPC.
                        // This will quietly drop any updates on functions that we no longer
                        // know/care about.
                        self.running_function_calls
                            .get(&rpc_data.context)
                            .map(|channel| {
                                // The other side of this channel might not exist anymore - we
                                // might have dropped the RPC already. Just ignore it.
                                let _ = channel.send(rpc_data);
                            });
                    },
                }
            },
            Command::Send(message) => {
                // NOCOM(#sirver): is this used?
                self.event_loop_sender.send(event_loop::Command::Send(message)).expect("Command::Send");
            },
            Command::OutgoingCall(context, tx, message) => {
                self.running_function_calls.insert(context, tx);
                // NOCOM(#sirver): can the message be constructed here?
                self.event_loop_sender.send(event_loop::Command::Send(message)).expect("Command::Call");
            }
            Command::CancelOutgoingRpc(context) => {
                let msg = ::ipc::Message::RpcCancel(::rpc::Cancel {
                    context: context,
                });
                self.event_loop_sender.send(event_loop::Command::Send(msg)).expect("Command::CancelOutgoingRpc");
            }
        };
        return false;
    }
}

pub fn spawn<'a>(commands: mpsc::Receiver<Command>,
                 command_sender: CommandSender,
                 new_rpcs: mpsc::Receiver<NewRpc<'a>>,
                 event_loop_sender: mio::Sender<event_loop::Command>) -> ::thread_scoped::JoinGuard<'a, ()>
{
    unsafe {
        ::thread_scoped::scoped(move || {
            let mut thread = RpcLoop {
                remote_procedures: HashMap::new(),
                running_function_calls: HashMap::new(),
                running_rpc_calls: HashMap::new(),
                event_loop_sender: event_loop_sender,
                command_sender: command_sender,
            };
            thread.spin_forever(commands, new_rpcs);
        })
    }
}
