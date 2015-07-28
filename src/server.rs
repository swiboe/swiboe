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

pub enum FunctionResult {
    DONE,
}

pub enum Command {
    Shutdown,
    RegisterFunction(Box<Function>),
    // NOCOM(#sirver): How can this be a proper struct?
    CallFunction(String, FunctionCallContext),
    PluginConnected(Plugin),
    PluginDisconnected(Plugin),
    Broadcast(json::value::Value),
}
pub type CommandSender = Sender<Command>;

// NOCOM(#sirver): Really a struct?
pub struct FunctionCallContext {
    // NOCOM(#sirver): maybe force this to be a uuid?
    pub context: String,
    pub args: json::value::Value,
    pub commands: CommandSender,
    // NOCOM(#sirver): needs some sort of backchannel? Needs to know who send it.
}

pub trait Function: Send {
    fn name(&self) -> &str;
    fn call(&self, context: FunctionCallContext) -> FunctionResult;
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct RemotePluginId {
    serial: u64,
    token: mio::Token,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Plugin {
    Remote(RemotePluginId),
}

pub struct SupremeServer {
    functions: HashMap<String, Box<Function>>,
    commands: Receiver<Command>,
    // NOCOM(#sirver): bad name :(
    remote_plugins: Vec<Plugin>,
    event_loop_channel: mio::Sender<HandlerMessage>,
}

impl SupremeServer {
    pub fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                Command::Shutdown => break,
                Command::RegisterFunction(handler) => self.register_function(0, handler),
                Command::CallFunction(name, call_context) => {
                    let function = self.functions.get(&name as &str).unwrap();
                    function.call(call_context);
                },
                Command::Broadcast(args) => {
                    // NOCOM(#sirver): broadcast to internal plugins.
                    let s = json::to_string(&args).unwrap();

                    // println!("#sirver broadcast args: {:#?}", args);
                    for plugin in &self.remote_plugins {
                        match *plugin {
                            Plugin::Remote(id) =>  {
                                self.event_loop_channel.send(HandlerMessage::SendData(
                                        id, s.clone())).unwrap();
                            }
                        }
                    }
                },
                Command::PluginConnected(plugin) => {
                    self.remote_plugins.push(plugin);
                },
                Command::PluginDisconnected(plugin) => {
                    let index = self.remote_plugins.position_elem(&plugin).unwrap();
                    self.remote_plugins.swap_remove(index);
                    // NOCOM(#sirver): needs to remove all associated functions
                }
            }
        }
    }

    // TODO(sirver): use priority.
    pub fn register_function(&mut self, _: u32, handler: Box<Function>) {
        self.functions.insert(handler.name().into(), handler);
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
                    let connection = Connection {
                        stream: stream,
                        remote_plugin_id: remote_plugin_id,
                    };
                    commands.send(Command::PluginConnected(Plugin::Remote(remote_plugin_id))).unwrap();
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
                        self.commands.send(Command::PluginDisconnected(Plugin::Remote(connection.remote_plugin_id))).unwrap();
                    }
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
                            args: args,
                            commands: self.commands.clone(),
                        };
                        self.commands.send(Command::CallFunction(name, call_context)).unwrap();
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
        remote_plugins: Vec::new(),
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
