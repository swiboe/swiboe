use mio;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use super::ipc;
use super::ipc_bridge;
use super::plugin::{PluginId, FunctionCallContext, Plugin, FunctionResult};
use super::plugin_core;
use super::plugin_buffer;

pub enum Command {
    Shutdown,
    RegisterFunction(PluginId, String),
    CallFunction(FunctionCallContext),
    PluginConnected(Box<Plugin>),
    PluginDisconnected(PluginId),
    Broadcast(ipc::Message),
}
pub type CommandSender = Sender<Command>;

pub struct Switchboard {
    functions: HashMap<String, PluginId>,
    commands: Receiver<Command>,
    plugins: HashMap<PluginId, Box<Plugin>>,
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
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
                    let plugin_id = self.functions.get(&call_context.rpc_call.function as &str).unwrap();
                    let context = call_context.rpc_call.context.clone();
                    let remote_id = match call_context.caller {
                        PluginId::Remote(r) => r,
                        _ => panic!("Local plugins should not call!"),
                    };

                    let result = {
                        let owner = &mut self.plugins.get_mut(&plugin_id).unwrap();
                        owner.call(call_context)
                    };
                    // NOCOM(#sirver): every plugin should be a remote_id now. Or we need a
                    // backchannel.
                    match result {
                        FunctionResult::NotHandled => {
                            // NOCOM(#sirver): immediately try the next contender
                            self.ipc_bridge_commands.send(
                                ipc_bridge::Command::SendData(
                                    remote_id,
                                    ipc::Message::RpcReply(
                                        ipc::RpcReply {
                                            context: context,
                                            state: ipc::RpcState::Done,
                                            result: ipc::RpcResultKind::NoHandler
                                        }))).unwrap();
                        },
                        FunctionResult::Delegated => {
                            // NOCOM(#sirver): wait for a reply (or timeout), then call the next
                            // contender.
                        }
                        FunctionResult::Handled => {
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                remote_id,
                                ipc::Message::RpcReply(ipc::RpcReply {
                                    context: context,
                                    state: ipc::RpcState::Done,
                                    result: ipc::RpcResultKind::Ok
                            }))).unwrap();
                        }
                    }
                },
                Command::Broadcast(message) => self.broadcast(&message),
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
        self.ipc_bridge_commands.send(ipc_bridge::Command::Quit).unwrap();
    }

    fn broadcast(&self, msg: &ipc::Message) {
        // TODO(sirver): This repeats serialization for each plugin again unnecessarily.
        for plugin in self.plugins.values() {
            plugin.send(msg);
        }
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
            commands: rx,
            ipc_bridge_commands: event_loop.channel(),
        };

        let mut ipc_brigde = ipc_bridge::IpcBridge::new(
            &mut event_loop, &server.socket_name, server.commands.clone());

        let switchboard_thread = thread::spawn(move || {
            switchboard.spin_forever();
        });

        server.event_loop_thread = Some(thread::spawn(move || {
            event_loop.run(&mut ipc_brigde).unwrap();
            switchboard_thread.join().unwrap();
        }));

        plugin_core::register(&server.commands);

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
