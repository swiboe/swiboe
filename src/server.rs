use mio;
use std::collections::{HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use super::ipc;
use super::ipc_bridge;
use super::plugin::{FunctionCallContext, Plugin};
use super::plugin_core;
use super::plugin_buffer;

const CORE_FUNCTIONS_PREFIX: &'static str = "core.";

pub enum Command {
    Shutdown,
    RegisterFunction(ipc_bridge::ClientId, String, u16),
    CallFunction(FunctionCallContext),
    FunctionReply(ipc::RpcReply),
    ClientConnected(Plugin),
    PluginDisconnected(ipc_bridge::ClientId),
}
pub type CommandSender = Sender<Command>;

#[derive(Debug)]
struct RegisteredFunction {
    client_id: ipc_bridge::ClientId,
    priority: u16,
}

struct RunningRpc {
    call_context: FunctionCallContext,
    last_index: usize,
}

pub struct Switchboard {
    functions: HashMap<String, Vec<RegisteredFunction>>,
    commands: Receiver<Command>,
    plugins: HashMap<ipc_bridge::ClientId, Plugin>,
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
                    // NOCOM(#sirver): make sure the plugin has not already registered this
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
                Command::CallFunction(call_context) => {
                    // NOCOM(#sirver): make sure this is not already in running_rpcs.
                    // NOCOM(#sirver): function name might not be in there.

                    // Special case 'core.'. We handle them immediately.
                    if call_context.rpc_call.function.starts_with(CORE_FUNCTIONS_PREFIX) {
                        let result = self.plugin_core.call(&call_context);
                        self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                call_context.caller,
                                ipc::Message::RpcReply(ipc::RpcReply {
                                    context: call_context.rpc_call.context.clone(),
                                    state: ipc::RpcState::Done,
                                    result: result,
                                }))).unwrap();
                    } else {
                        let vec = self.functions.get(&call_context.rpc_call.function as &str).unwrap();
                        let function = &vec[0];

                        let owner = &mut self.plugins.get_mut(&function.client_id).unwrap();
                        owner.call(&call_context);
                        self.running_rpcs.insert(call_context.rpc_call.context.clone(), RunningRpc {
                            last_index: 0,
                            call_context: call_context,
                        });
                        // NOCOM(#sirver): we ignore timeouts.
                    }
                },
                Command::FunctionReply(rpc_reply) => {
                    // NOCOM(#sirver): what if the context is unknown? drop the client?

                    // NOCOM(#sirver): maybe not remove, but mutate?
                    let mut running_rpc = self.running_rpcs.remove(&rpc_reply.context).unwrap();

                    match rpc_reply.result {
                        // NOCOM(#sirver): same for errors.
                        ipc::RpcResult::Ok(_) => {
                            // NOCOM(#sirver): done, remove this call.
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                    running_rpc.call_context.caller,
                                    ipc::Message::RpcReply(rpc_reply))).unwrap();
                        },
                        ipc::RpcResult::NotHandled => {
                            // TODO(sirver): If a new function has been registered or been deleted since we
                            // last saw this context, this might skip a handler or call one twice. We need
                            // a better way to keep track where we are in the list of handlers.
                            let function = {
                                running_rpc.last_index += 1;
                                // NOCOM(#sirver): quite some code duplication with CallFunction
                                let vec = self.functions.get(&running_rpc.call_context.rpc_call.function as &str).unwrap();
                                match vec.get(running_rpc.last_index) {
                                    Some(function) => function,
                                    None => {
                                        // NOCOM(#sirver): return that it was not handled.
                                        unimplemented!();
                                    }
                                }
                            };

                            let owner = &mut self.plugins.get_mut(&function.client_id).unwrap();
                            owner.call(&running_rpc.call_context);
                            self.running_rpcs.insert(running_rpc.call_context.rpc_call.context.clone(), running_rpc);
                            // NOCOM(#sirver): we ignore timeouts.
                        }
                    }
                },
                Command::ClientConnected(plugin) => {
                    // NOCOM(#sirver): make sure plugin is not yet known.
                    self.plugins.insert(plugin.client_id(), plugin);
                },
                Command::PluginDisconnected(plugin) => {
                    self.plugins.remove(&plugin).unwrap();
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
            plugins: HashMap::new(),
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
