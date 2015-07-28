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

pub struct SupremeServer {
    functions: HashMap<String, Box<Function>>,
    commands: Receiver<Command>,
    // NOCOM(#sirver): bad name :(
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

                    // println!("#sirver broadcast args: {:#?}", args);
                    self.event_loop_channel.send(
                        HandlerMessage::Broadcast(args));
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
    // NOCOM(#sirver): replace conn and conns
    connections: mio::util::Slab<Connection>,
    commands: CommandSender,
}

pub struct Connection {
    stream: UnixStream,
    token: mio::Token,
}

// NOCOM(#sirver): ClientConnection?
impl Connection {
    pub fn new(stream: UnixStream, token: mio::Token) -> Self {
        Connection {
            stream: stream,
            token: token,
        }
    }
}

enum HandlerMessage {
    Quit,
    Broadcast(json::value::Value),
}

impl mio::Handler for IpcBridge {
    type Timeout = ();
    type Message = HandlerMessage;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, message: HandlerMessage) {
        match message {
            HandlerMessage::Quit => event_loop.shutdown(),
            HandlerMessage::Broadcast(data) => {
                let s = json::to_string(&data).unwrap();
                for conn in self.connections.iter_mut() {
                    conn.stream.write_message(s.as_bytes());
                }
            }
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            SERVER => {
                let stream = self.unix_listener.accept().unwrap().unwrap();
                match self.connections.insert_with(|token| {
                    println!("registering {:?} with event loop", token);
                    Connection::new(stream, token)
                }) {
                    Some(token) => {
                        // If we successfully insert, then register our connection.
                        let conn = &mut self.connections[token];
                        event_loop.register_opt(
                            &conn.stream,
                            conn.token,
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
    };
    let mut s_server = SupremeServer {
        functions: HashMap::new(),
        commands: rx,
        event_loop_channel: event_loop.channel(),
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
