use mio::unix::{UnixListener, UnixStream};
use mio;
use serde::json;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use super::buffer::Buffer;
use super::ipc::{IpcRead, IpcWrite};
use super::plugin_core;

const SERVER: mio::Token = mio::Token(0);

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub struct RemotePluginId {
    serial: u64,
    token: mio::Token,
}

#[derive(PartialEq, Eq, Clone, Copy, Hash)]
pub enum PluginId {
    // NOCOM(#sirver): rename to internal.
    Local(&'static str),
    Remote(RemotePluginId),
}

pub trait Plugin: Send {
    fn name(&self) -> &'static str;
    fn id(&self) -> PluginId;
    fn broadcast(&self, data: &json::value::Value);
    fn call(&self, context: FunctionCallContext) -> FunctionResult;
}

struct RemotePlugin {
    id: PluginId,
    event_loop_channel: mio::Sender<HandlerMessage>,
}

impl RemotePlugin {
    fn remote_id(&self) -> RemotePluginId {
        if let PluginId::Remote(remote_id) = self.id {
            return remote_id;
        }
        panic!("RemotePlugin with non ::Remote() id.");
    }
}

impl Plugin for RemotePlugin {
    // NOCOM(#sirver): name does not fit :(
    fn name(&self) -> &'static str { "remote_plugin" }
    fn id(&self) -> PluginId {
        self.id
    }

    fn broadcast(&self, data: &json::value::Value) {
        let s = json::to_string(&data).unwrap();
        self.event_loop_channel.send(
            HandlerMessage::SendData(self.remote_id(), s)).unwrap();
    }

    fn call(&self, context: FunctionCallContext) -> FunctionResult {
        let data = json::builder::ObjectBuilder::new()
                .insert("context".into(), context.context)
                .insert("function".into(), context.function)
                .insert("args".into(), context.args)
                .unwrap();
        let s = json::to_string(&data).unwrap();
        self.event_loop_channel.send(
            HandlerMessage::SendData(self.remote_id(), s)).unwrap();
        FunctionResult::DONE
    }
}

pub enum FunctionResult {
    DONE,
}

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

// NOCOM(#sirver): Really a struct?
pub struct FunctionCallContext {
    // NOCOM(#sirver): maybe force this to be a uuid?
    pub context: String,
    pub function: String,
    pub args: json::value::Value,
    pub commands: CommandSender,
    pub caller: PluginId,
    // NOCOM(#sirver): needs some sort of backchannel? Needs to know who send it.
}

// NOCOM(#sirver): can go away?
pub trait Function: Send {
    fn name(&self) -> &str;
    fn call(&self, context: FunctionCallContext) -> FunctionResult;
}

struct FunctionRegister {
    owner: PluginId,
    handler: Box<Function>,
}

pub struct SupremeServer {
    functions: HashMap<String, PluginId>,
    commands: Receiver<Command>,
    // NOCOM(#sirver): rather a hashmap?
    plugins: HashMap<PluginId, Box<Plugin>>,
    event_loop_channel: mio::Sender<HandlerMessage>,
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

struct IpcBridge {
    unix_listener: UnixListener,
    connections: mio::util::Slab<Connection>,
    commands: CommandSender,
    next_serial: u64,
}

// NOCOM(#sirver): ClientConnection?
pub struct Connection {
    stream: UnixStream,
    remote_plugin_id: RemotePluginId,
}

enum HandlerMessage {
    Quit,
    SendData(RemotePluginId, String),
}

impl mio::Handler for IpcBridge {
    type Timeout = ();
    type Message = HandlerMessage;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, message: HandlerMessage) {
        match message {
            HandlerMessage::Quit => event_loop.shutdown(),
            HandlerMessage::SendData(receiver, data) => {
                // NOCOM(#sirver): what if that is no longer valid?
                let conn = &mut self.connections[receiver.token];
                if conn.remote_plugin_id == receiver {
                    conn.stream.write_message(data.as_bytes());
                }
            }
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            SERVER => {
                let stream = self.unix_listener.accept().unwrap().unwrap();
                // NOCOM(#sirver): can this be done in Some(token)?
                let serial = self.next_serial;
                let commands = self.commands.clone();
                self.next_serial += 1;
                match self.connections.insert_with(|token| {
                    println!("registering {:?} with event loop", token);
                    let remote_plugin_id = RemotePluginId {
                        serial: serial,
                        token: token,
                    };
                    let plugin = RemotePlugin {
                        id: PluginId::Remote(remote_plugin_id),
                        event_loop_channel: event_loop.channel(),
                    };
                    let connection = Connection {
                        stream: stream,
                        remote_plugin_id: remote_plugin_id,
                    };
                    commands.send(Command::PluginConnected(Box::new(plugin))).unwrap();
                    connection
                }) {
                    Some(token) => {
                        // If we successfully insert, then register our connection.
                        let conn = &mut self.connections[token];
                        event_loop.register_opt(
                            &conn.stream,
                            conn.remote_plugin_id.token,
                            mio::EventSet::readable(),
                            mio::PollOpt::level()).unwrap();
                    },
                    None => {
                        // If we fail to insert, `conn` will go out of scope and be dropped.
                        panic!("Failed to insert connection into slab");
                    }
                };
            },
            client_token => {
                if events.is_hup() {
                    {
                        let connection = &self.connections[client_token];
                        self.commands.send(Command::PluginDisconnected(PluginId::Remote(connection.remote_plugin_id))).unwrap();
                    }
                    // NOCOM(#sirver): does this return the entry? If so, code can be simplified.
                    self.connections.remove(client_token);
                } else if events.is_readable() {
                    let mut vec = Vec::new();
                    let conn = &mut self.connections[token];
                    // NOCOM(#sirver): read_message can read into a string directly?
                    conn.stream.read_message(&mut vec);
                    let s = String::from_utf8(vec).unwrap();
                    let value: json::Value = json::from_str(&s).unwrap();
                    if value.find("type").and_then(|o| o.as_string()) == Some("call") {
                        let name = value.find("function")
                            .and_then(|o| o.as_string()).unwrap().into();
                        let context = value.find("context")
                            .and_then(|o| o.as_string()).unwrap().into();
                        let args = value.find("args")
                            .map(|args| args.clone())
                            .unwrap_or(json::builder::ObjectBuilder::new().unwrap());
                        let call_context = FunctionCallContext {
                            context: context,
                            function: name,
                            args: args,
                            commands: self.commands.clone(),
                            caller: PluginId::Remote(conn.remote_plugin_id),
                        };
                        self.commands.send(Command::CallFunction(call_context)).unwrap();
                    }
                }
            }
        }
    }
}

pub fn run_supreme_server() {
    // TODO(sirver): grep for unwrap and remove
    let server = UnixListener::bind("/tmp/s.socket").unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER).unwrap();

    let (tx, rx) = channel();
    let mut ipc_brigde = IpcBridge {
        unix_listener: server,
        connections: mio::util::Slab::new_starting_at(mio::Token(1), 1024),
        commands: tx.clone(),
        next_serial: 1,
    };
    let mut s_server = SupremeServer {
        functions: HashMap::new(),
        commands: rx,
        event_loop_channel: event_loop.channel(),
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
