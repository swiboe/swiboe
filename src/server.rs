use ::error::Error;
use ::ipc;
use ::ipc_bridge;
use ::plugin_buffer;
use ::plugin_core;
use ::plugin_list_files;
use ::rpc;
use mio;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

// NOCOM(#sirver): when a client disconnects and we still try to call one of it's rpcs, we never
// get an error back - this will effectively interrupt the rpc call stack.
// NOCOM(#sirver): document everything.
const CORE_FUNCTIONS_PREFIX: &'static str = "core.";

pub enum Command {
    Quit,
    NewRpc(ipc_bridge::ClientId, String, u16),
    RpcCall(ipc_bridge::ClientId, rpc::Call),
    RpcResponse(rpc::Response),
    RpcCancel(rpc::Cancel),
    ClientConnected(ipc_bridge::ClientId),
    ClientDisconnected(ipc_bridge::ClientId),
    SendDataFailed(ipc_bridge::ClientId, ipc::Message, Error),
}
pub type CommandSender = Sender<Command>;

#[derive(Debug)]
struct RegisteredFunction {
    client_id: ipc_bridge::ClientId,
    priority: u16,
}

#[derive(Debug)]
struct RunningRpc {
    caller: ipc_bridge::ClientId,
    rpc_call: rpc::Call,
    last_index: usize,
}

pub struct Swiboe {
    functions: HashMap<String, Vec<RegisteredFunction>>,
    commands: Receiver<Command>,
    clients: HashSet<ipc_bridge::ClientId>,
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
    running_rpcs: HashMap<String, RunningRpc>,
    plugin_core: plugin_core::CorePlugin,
}

impl Swiboe {
    pub fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                Command::Quit => break,
                Command::NewRpc(client_id, name, priority) => {
                    // NOCOM(#sirver): deny everything starting with 'core'
                    // NOCOM(#sirver): make sure the client_id is known.
                    // NOCOM(#sirver): make sure the client has not already registered this
                    // function.
                    let vec = self.functions.entry(name)
                        .or_insert(Vec::new());

                    let index = match vec.binary_search_by(|probe| probe.priority.cmp(&priority)) {
                        Ok(idx) => idx,
                        Err(idx) => idx,
                    };

                    vec.insert(index, RegisteredFunction {
                        client_id: client_id,
                        priority: priority,
                    });
                },
                Command::RpcCall(client_id, rpc_call) => {
                    // NOCOM(#sirver): make sure this is not already in running_rpcs.
                    // NOCOM(#sirver): function name might not be in there.

                    // Special case 'core.'. We handle them immediately.
                    if rpc_call.function.starts_with(CORE_FUNCTIONS_PREFIX) {
                        let result = self.plugin_core.call(client_id, &rpc_call);
                        self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                client_id,
                                ipc::Message::RpcResponse(rpc::Response {
                                    context: rpc_call.context.clone(),
                                    kind: rpc::ResponseKind::Last(result),
                                }))).unwrap();
                    } else {
                        match self.functions.get(&rpc_call.function as &str) {
                            Some(vec) => {
                                let function = &vec[0];
                                // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                                // able to move again.
                                self.running_rpcs.insert(rpc_call.context.clone(), RunningRpc {
                                    last_index: 0,
                                    rpc_call: rpc_call.clone(),
                                    caller: client_id,
                                });
                                self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                        function.client_id,
                                        ipc::Message::RpcCall(rpc_call)
                                        )).unwrap();
                                // NOCOM(#sirver): we ignore timeouts.
                            },
                            None => {
                                self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                        client_id,
                                        ipc::Message::RpcResponse(rpc::Response {
                                            context: rpc_call.context.clone(),
                                            kind: rpc::ResponseKind::Last(rpc::Result::Err(rpc::Error {
                                                kind: rpc::ErrorKind::UnknownRpc,
                                                details: None,
                                            })),
                                        }))).unwrap();
                            }
                        }
                    }
                },
                Command::RpcResponse(rpc_response) => {
                    self.on_rpc_response(rpc_response)
                },
                Command::RpcCancel(rpc_cancel) => {
                    self.on_rpc_cancel(rpc_cancel)
                },
                Command::SendDataFailed(client_id, msg, err) => {
                    let action = match msg {
                        ipc::Message::RpcResponse(_) | ipc::Message::RpcCancel(_) => {
                            // NOCOM(#sirver): on a streaming rpc, this should also try to cancel
                            // the RPC.
                            "dropped the RpcResponse/RpcCall."
                        },
                        ipc::Message::RpcCall(rpc_call) => {
                            self.on_rpc_response(rpc::Response {
                                context: rpc_call.context,
                                kind: rpc::ResponseKind::Last(rpc::Result::NotHandled),
                            });
                            "surrogate replied as NotHandled."
                        }
                    };
                    println!("Sending to {:?} failed: {:?}, {}", client_id, err, action);
                },
                Command::ClientConnected(client_id) => {
                    // NOCOM(#sirver): make sure client_id is not yet known.
                    self.clients.insert(client_id);
                },
                Command::ClientDisconnected(client_id) => {
                    self.clients.remove(&client_id);

                    // Kill all pending RPCs that have been requested by this client.
                    let rpcs_to_remove: Vec<_> = self.running_rpcs.iter()
                        .filter_map(|(context, running_rpc)| {
                            if running_rpc.caller == client_id {
                                Some(context.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    for context in rpcs_to_remove {
                        self.running_rpcs.remove(&context);
                    }

                    // Kill all functions that have been registered by this.
                    let mut functions_to_remove = Vec::new();
                    for (function_name, registered_functions) in &mut self.functions {
                        registered_functions.retain(|registered_function| {
                            registered_function.client_id != client_id
                        });
                        if registered_functions.is_empty() {
                            functions_to_remove.push(function_name.to_string());
                        }
                    }
                    for function_name in functions_to_remove {
                        self.functions.remove(&function_name);
                    }
                }
            }
        }
    }

    fn on_rpc_cancel(&mut self, rpc_cancel: rpc::Cancel) {
        let running_rpc = match self.running_rpcs.entry(rpc_cancel.context.clone()) {
            Entry::Occupied(running_rpc) => running_rpc,
            Entry::Vacant(_) => {
                // Unknown RPC. We simply drop this message.
                return;
            }
        };

        // NOCOM(#sirver): only the original caller can cancel, really.
        let running_rpc = running_rpc.remove();
        match {
            // NOCOM(#sirver): quite some code duplication with RpcCall
            self.functions.get(&running_rpc.rpc_call.function as &str).and_then(|vec| {
                vec.get(running_rpc.last_index)
            })
        } {
            Some(function) => {
                // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                // able to move again.
                self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                        function.client_id,
                        ipc::Message::RpcCancel(rpc_cancel)
                        )).unwrap();
            },
            None => {
                // NOCOM(#sirver): Wait what... nothing to cancel?
            }
        };
    }

    fn on_rpc_response(&mut self, rpc_response: rpc::Response) {
        let mut running_rpc = match self.running_rpcs.entry(rpc_response.context.clone()) {
            Entry::Occupied(running_rpc) => running_rpc,
            Entry::Vacant(_) => {
                // Unknown RPC. We simply drop this message.
                return;
            }
        };

        match rpc_response.kind {
            rpc::ResponseKind::Partial(value) => {
                let running_rpc = running_rpc.get();
                self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                        running_rpc.caller,
                        ipc::Message::RpcResponse(rpc::Response {
                            context: running_rpc.rpc_call.context.clone(),
                            kind: rpc::ResponseKind::Partial(value),
                        }))).unwrap();
            },
            rpc::ResponseKind::Last(result) => match result {
                rpc::Result::Ok(_) | rpc::Result::Err(_) => {
                    let running_rpc = running_rpc.remove();
                    self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                            running_rpc.caller,
                            ipc::Message::RpcResponse(rpc::Response {
                                context: running_rpc.rpc_call.context,
                                kind: rpc::ResponseKind::Last(
                                    result
                                ),
                            }))).unwrap();
                },
                rpc::Result::NotHandled => {
                    // TODO(sirver): If a new function has been registered or been deleted since we
                    // last saw this context, this might skip a handler or call one twice. We need
                    // a better way to keep track where we are in the list of handlers.
                    let running_rpc = running_rpc.get_mut();


                    running_rpc.last_index += 1;
                    match {
                        // NOCOM(#sirver): quite some code duplication with RpcCall
                        self.functions.get(&running_rpc.rpc_call.function as &str).and_then(|vec| {
                            vec.get(running_rpc.last_index)
                        })
                    } {
                        Some(function) => {
                            // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                            // able to move again.
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                    function.client_id,
                                    ipc::Message::RpcCall(running_rpc.rpc_call.clone())
                                    )).unwrap();
                        },
                        None => {
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                    running_rpc.caller,
                                    ipc::Message::RpcResponse(rpc::Response {
                                        context: running_rpc.rpc_call.context.clone(),
                                        kind: rpc::ResponseKind::Last(rpc::Result::NotHandled),
                                    }))).unwrap();
                        }
                    };
                    // NOCOM(#sirver): we ignore timeouts.
                }
            },
        }
    }
}

pub struct Server {
    socket_name: PathBuf,
    commands: CommandSender,
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
    swiboe_thread: Option<thread::JoinHandle<()>>,
    event_loop_thread: Option<thread::JoinHandle<()>>,
    buffer_plugin: Option<plugin_buffer::BufferPlugin>,
    list_files_plugin: Option<plugin_list_files::ListFilesPlugin>,
}

impl Server {
    pub fn launch(socket_name: &Path) -> Self {
        let (tx, rx) = channel();

        // TODO(sirver): grep for unwrap and remove
        let mut event_loop = mio::EventLoop::new().unwrap();

        let mut server = Server {
            socket_name: socket_name.to_path_buf(),
            commands: tx,
            ipc_bridge_commands: event_loop.channel(),
            buffer_plugin: None,
            list_files_plugin: None,
            swiboe_thread: None,
            event_loop_thread: None,
        };

        let mut swiboe = Swiboe {
            functions: HashMap::new(),
            clients: HashSet::new(),
            running_rpcs: HashMap::new(),
            commands: rx,
            ipc_bridge_commands: event_loop.channel(),
            plugin_core: plugin_core::CorePlugin::new(server.commands.clone()),
        };

        let mut ipc_bridge = ipc_bridge::IpcBridge::new(
            &mut event_loop, &server.socket_name, server.commands.clone());

        server.event_loop_thread = Some(thread::spawn(move || {
            event_loop.run(&mut ipc_bridge).unwrap();
        }));

        server.swiboe_thread = Some(thread::spawn(move || {
            swiboe.spin_forever();
        }));

        server.buffer_plugin = Some(
            plugin_buffer::BufferPlugin::new(&server.socket_name));
        server.list_files_plugin = Some(
            plugin_list_files::ListFilesPlugin::new(&server.socket_name));
        server
    }

    pub fn shutdown(&mut self) {
        // Any of the threads might have already panicked. So we ignore send errors.
        let _ = self.ipc_bridge_commands.send(ipc_bridge::Command::Quit);
        let _ = self.commands.send(Command::Quit);
        self.wait_for_shutdown();
    }

    pub fn wait_for_shutdown(&mut self) {
        if let Some(thread) = self.swiboe_thread.take() {
            thread.join().expect("Could not join swiboe_thread.");
        }
        if let Some(thread) = self.event_loop_thread.take() {
            thread.join().expect("Could not join event_loop_thread.");
        }

        fs::remove_file(&self.socket_name).expect(
            &format!("Could not remove socket {:?}", self.socket_name));
    }
}
