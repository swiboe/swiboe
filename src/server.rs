use mio;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use super::ipc;
use super::ipc_bridge;
use super::plugin_buffer;
use super::plugin_core;

// NOCOM(#sirver): document everything.
const CORE_FUNCTIONS_PREFIX: &'static str = "core.";

pub enum Command {
    Shutdown,
    RegisterFunction(ipc_bridge::ClientId, String, u16),
    CallFunction(ipc_bridge::ClientId, ipc::RpcCall),
    FunctionReply(ipc::RpcReply),
    ClientConnected(ipc_bridge::ClientId),
    ClientDisconnected(ipc_bridge::ClientId),
}
pub type CommandSender = Sender<Command>;

#[derive(Debug)]
struct RegisteredFunction {
    client_id: ipc_bridge::ClientId,
    priority: u16,
}

struct RunningRpc {
    caller: ipc_bridge::ClientId,
    rpc_call: ipc::RpcCall,
    last_index: usize,
}

pub struct Switchboard {
    functions: HashMap<String, Vec<RegisteredFunction>>,
    commands: Receiver<Command>,
    clients: HashSet<ipc_bridge::ClientId>,
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
    running_rpcs: HashMap<String, RunningRpc>,
    plugin_core: plugin_core::CorePlugin,
}

impl Switchboard {
    pub fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                Command::Shutdown => break,
                Command::RegisterFunction(client_id, name, priority) => {
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
                Command::CallFunction(client_id, rpc_call) => {
                    // NOCOM(#sirver): make sure this is not already in running_rpcs.
                    // NOCOM(#sirver): function name might not be in there.

                    // Special case 'core.'. We handle them immediately.
                    if rpc_call.function.starts_with(CORE_FUNCTIONS_PREFIX) {
                        let result = self.plugin_core.call(client_id, &rpc_call);
                        self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                client_id,
                                ipc::Message::RpcReply(ipc::RpcReply {
                                    context: rpc_call.context.clone(),
                                    state: ipc::RpcState::Done,
                                    result: result,
                                }))).unwrap();
                    } else {
                        let vec = self.functions.get(&rpc_call.function as &str).unwrap();
                        let function = &vec[0];

                        // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                        // able to move again.
                        self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                function.client_id,
                                ipc::Message::RpcCall(rpc_call.clone())
                        )).unwrap();
                        self.running_rpcs.insert(rpc_call.context.clone(), RunningRpc {
                            last_index: 0,
                            rpc_call: rpc_call,
                            caller: client_id,
                        });
                        // NOCOM(#sirver): we ignore timeouts.
                    }
                },
                Command::FunctionReply(rpc_reply) => {

                    // NOCOM(#sirver): maybe not remove, but mutate?
                    let mut running_rpc = match self.running_rpcs.entry(rpc_reply.context.clone()) {
                        Entry::Occupied(running_rpc) => running_rpc,
                        Entry::Vacant(_) => {
                            // NOCOM(#sirver): what if the context is unknown? drop the client?
                            unimplemented!();
                        }
                    };

                    match rpc_reply.result {
                        // NOCOM(#sirver): same for errors.
                        ipc::RpcResult::Ok(_) => {
                            let running_rpc = running_rpc.remove();
                            // NOCOM(#sirver): done, remove this call.
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                    running_rpc.caller,
                                    ipc::Message::RpcReply(rpc_reply))).unwrap();
                        },
                        ipc::RpcResult::NotHandled => {
                            // TODO(sirver): If a new function has been registered or been deleted since we
                            // last saw this context, this might skip a handler or call one twice. We need
                            // a better way to keep track where we are in the list of handlers.
                            let running_rpc = running_rpc.get_mut();
                            let function = {
                                running_rpc.last_index += 1;
                                // NOCOM(#sirver): quite some code duplication with CallFunction
                                let vec = self.functions.get(&running_rpc.rpc_call.function as &str).unwrap();
                                match vec.get(running_rpc.last_index) {
                                    Some(function) => function,
                                    None => {
                                        // NOCOM(#sirver): return that it was not handled.
                                        unimplemented!();
                                    }
                                }
                            };

                            // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                            // able to move again.
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                    function.client_id,
                                    ipc::Message::RpcCall(running_rpc.rpc_call.clone())
                            )).unwrap();

                            // self.running_rpcs.insert(running_rpc.rpc_call.context.clone(), running_rpc);
                            // NOCOM(#sirver): we ignore timeouts.
                        }
                    }
                },
                Command::ClientConnected(client_id) => {
                    // NOCOM(#sirver): make sure client_id is not yet known.
                    self.clients.insert(client_id);
                },
                Command::ClientDisconnected(client_id) => {
                    self.clients.remove(&client_id);
                    // NOCOM(#sirver): needs to remove all associated functions
                }
            }
        }
        self.ipc_bridge_commands.send(ipc_bridge::Command::Quit).unwrap();
    }
}

pub struct Server<'a> {
    socket_name: PathBuf,
    commands: CommandSender,
    event_loop_thread: Option<thread::JoinHandle<()>>,
    buffer_plugin: Option<plugin_buffer::BufferPlugin<'a>>,
}

impl<'a> Server<'a> {
    pub fn launch(socket_name: &Path) -> Self {
        let (tx, rx) = channel();

        let mut server = Server {
            socket_name: socket_name.to_path_buf(),
            commands: tx,
            event_loop_thread: None,
            buffer_plugin: None,
        };

        // TODO(sirver): grep for unwrap and remove
        let mut event_loop = mio::EventLoop::new().unwrap();

        let mut switchboard = Switchboard {
            functions: HashMap::new(),
            clients: HashSet::new(),
            running_rpcs: HashMap::new(),
            commands: rx,
            ipc_bridge_commands: event_loop.channel(),
            plugin_core: plugin_core::CorePlugin::new(server.commands.clone()),
        };

        let mut ipc_bridge = ipc_bridge::IpcBridge::new(
            &mut event_loop, &server.socket_name, server.commands.clone());

        let switchboard_thread = thread::spawn(move || {
            switchboard.spin_forever();
        });

        server.event_loop_thread = Some(thread::spawn(move || {
            event_loop.run(&mut ipc_bridge).unwrap();
            switchboard_thread.join().unwrap();
        }));

        server.buffer_plugin = Some(
            plugin_buffer::BufferPlugin::new(&server.socket_name));
        server
    }

    pub fn shutdown(&mut self) {
        self.commands.send(Command::Shutdown).unwrap();
        self.wait_for_shutdown();
    }

    pub fn wait_for_shutdown(&mut self) {
        if let Some(thread) = self.event_loop_thread.take() {
            thread.join().expect("Could not join event_loop_thread.");
            fs::remove_file(&self.socket_name).expect(
                &format!("Could not remove socket {:?}", self.socket_name));
        }
    }
}
