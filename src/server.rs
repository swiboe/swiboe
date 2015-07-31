use mio;
use serde::json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use super::ipc_bridge;
use super::plugin::{PluginId, FunctionCallContext, Plugin, FunctionResult};
use super::plugin_core;

pub enum Command {
    Shutdown,
    RegisterFunction(PluginId, String),
    CallFunction(FunctionCallContext),
    PluginConnected(Box<Plugin>),
    PluginDisconnected(PluginId),
    Broadcast(json::value::Value),
}
pub type CommandSender = Sender<Command>;

pub struct Switchboard {
    functions: HashMap<String, PluginId>,
    commands: Receiver<Command>,
    plugins: HashMap<PluginId, Box<Plugin>>,
}

// NOCOM(#sirver): more compact custom serialization?
// NOCOM(#sirver): use actual result type?
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum RpcResultKind {
    Ok,
    NoHandler,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum RpcState {
    Running,
    Done,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcReply {
    pub context: String,
    pub state: RpcState,
    pub result: RpcResultKind,
}

impl Switchboard {
    pub fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                Command::Shutdown => break,
                Command::RegisterFunction(plugin_id, name) => {
                    // NOCOM(#sirver): make sure the plugin_id is known.
                    // TODO(sirver): add priority.
                    self.functions.insert(name, plugin_id);
                },
                Command::CallFunction(call_context) => {
                    let plugin_id = self.functions.get(&call_context.function as &str).unwrap();
                    let context = call_context.context.clone();
                    let result = {
                        let owner = &mut self.plugins.get_mut(&plugin_id).unwrap();
                        owner.call(call_context)
                    };
                    match result {
                        FunctionResult::NotHandled => {
                            // NOCOM(#sirver): immediately try the next contender
                            self.broadcast(&json::to_value(&RpcReply {
                                context: context,
                                state: RpcState::Done,
                                result: RpcResultKind::NoHandler
                            }));
                        },
                        FunctionResult::Delegated => {
                            // NOCOM(#sirver): wait for a reply (or timeout), then call the next
                            // contender.
                        }
                        FunctionResult::Handled => {
                            self.broadcast(&json::to_value(&RpcReply {
                                context: context,
                                state: RpcState::Done,
                                result: RpcResultKind::Ok
                            }));
                        }
                    }
                },
                Command::Broadcast(args) => self.broadcast(&args),
                Command::PluginConnected(plugin) => {
                    // NOCOM(#sirver): make sure plugin is not yet known.
                    self.plugins.insert(plugin.id(), plugin);
                },
                Command::PluginDisconnected(plugin) => {
                    self.plugins.remove(&plugin).unwrap();
                    // NOCOM(#sirver): needs to remove all associated functions
                }
            }
        }
    }

    fn broadcast(&self, data: &json::Value) {
        // NOCOM(#sirver): repeats serialization. :(
        for plugin in self.plugins.values() {
            plugin.broadcast(&data);
        }
    }

    // NOCOM(#sirver): nuke?
    // fn broadcast_rpc_result(&self, result: RpcResult, context: &str) {
        // let result_str = match result {
            // RpcResult::Ok => { "ok" },
            // RpcResult::NoHandler => { "no_handler" },
        // };

        // let data = json::builder::ObjectBuilder::new()
                // .insert("context".into(), context)
                // .insert("result".into(), result_str)
                // .unwrap();
        // self.broadcast(&data);
    // }
}

pub struct Server {
    event_loop_thread: Option<thread::JoinHandle<()>>,
    commands: CommandSender,
    socket_name: PathBuf,
}

impl Server {
    pub fn launch(socket_name: &Path) -> Self {
        // TODO(sirver): grep for unwrap and remove
        let mut event_loop = mio::EventLoop::new().unwrap();

        let (tx, rx) = channel();
        let mut ipc_brigde = ipc_bridge::IpcBridge::new(
            &mut event_loop, socket_name, tx.clone());

        let mut s_server = Switchboard {
            functions: HashMap::new(),
            commands: rx,
            plugins: HashMap::new(),
        };

        plugin_core::register(&tx);

        let ipc_bridge_commands = event_loop.channel();
        let switchboard_thread = thread::spawn(move || {
            s_server.spin_forever();
            ipc_bridge_commands.send(ipc_bridge::Command::Quit).unwrap();
        });

        let event_loop_thread = thread::spawn(move || {
            event_loop.run(&mut ipc_brigde).unwrap();
            switchboard_thread.join().unwrap();
        });

        Server {
            event_loop_thread: Some(event_loop_thread),
            commands: tx,
            socket_name: socket_name.to_path_buf(),
        }
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
