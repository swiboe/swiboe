use mio;
use serde::json;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use super::plugin_base::{PluginId, FunctionCallContext, Plugin};
use super::ipc_bridge::{IpcBridge, HandlerMessage};
use super::plugin_core;

pub enum Command {
    Shutdown,
    RegisterFunction(PluginId, String),
    // NOCOM(#sirver): How can this be a proper struct?
    CallFunction(FunctionCallContext),
    PluginConnected(Box<Plugin>),
    PluginDisconnected(PluginId),
    Broadcast(json::value::Value),
}
pub type CommandSender = Sender<Command>;

pub struct SupremeServer {
    functions: HashMap<String, PluginId>,
    commands: Receiver<Command>,
    // NOCOM(#sirver): rather a hashmap?
    plugins: HashMap<PluginId, Box<Plugin>>,
}

impl SupremeServer {
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
                    // NOCOM(#sirver): redo
                    let plugin_id = self.functions.get(&call_context.function as &str).unwrap();
                    let owner = &mut self.plugins.get_mut(&plugin_id).unwrap();
                    owner.call(call_context);
                },
                Command::Broadcast(args) => {
                    // NOCOM(#sirver): repeats serialization. :(
                    for plugin in self.plugins.values() {
                        plugin.broadcast(&args);
                    }
                },
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
}

pub fn run_supreme_server() {
    // TODO(sirver): grep for unwrap and remove
    let mut event_loop = mio::EventLoop::new().unwrap();

    let (tx, rx) = channel();
    let mut ipc_brigde = IpcBridge::new(&mut event_loop, "/tmp/s.socket", tx.clone());

    let mut s_server = SupremeServer {
        functions: HashMap::new(),
        commands: rx,
        plugins: HashMap::new(),
    };

    plugin_core::register(&tx);


    let event_loop_channel = event_loop.channel();
    let worker_thread = thread::spawn(move || {
        s_server.spin_forever();
        event_loop_channel.send(HandlerMessage::Quit).unwrap();
    });

    event_loop.run(&mut ipc_brigde).unwrap();
    worker_thread.join().unwrap();
}
