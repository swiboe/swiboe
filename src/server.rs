use mio::unix::{UnixListener, UnixStream};
use mio;
use serde::json;
use std::collections::HashMap;
use super::buffer::Buffer;
use super::ipc::{IpcRead, IpcWrite};
use super::plugin_core;

const SERVER: mio::Token = mio::Token(0);

pub enum FunctionResult {
    DONE,
    NOT_HANDLED,
}

pub trait Function<'a> {
    fn name(&self) -> &'a str;
    fn call(&self, args: &json::value::Value) -> FunctionResult;
}

pub struct SupremeServer<'a> {
    buffers: Vec<Buffer>,
    unix_listener: UnixListener,
    // NOCOM(#sirver): replace conn and conns
    conns: mio::util::Slab<Connection>,
    functions: HashMap<&'a str, Box<Function<'a>>>,
}

struct Connection {
    stream: UnixStream,
    token: mio::Token,
}

// NOCOM(#sirver): ClientConnection?
impl Connection {
    pub fn new(stream: UnixStream, token: mio::Token) -> Connection {
        Connection {
            stream: stream,
            token: token,
        }
    }
}

impl<'a> SupremeServer<'a> {
    pub fn shutdown(&mut self) {
        // self.event_loop.shutdown();
        println!("#sirver Should shutdown. ");
        // NOCOM(#sirver): figure that out
        // self.unix_listener.close();
    }

    // TODO(sirver): use priority.
    pub fn register_function(&mut self, unused_priority: u32, handler: Box<Function<'a>>) {
        self.functions.insert(handler.name(), handler);
    }
}

impl<'a> mio::Handler for SupremeServer<'a> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<SupremeServer>, token: mio::Token, events: mio::EventSet) {
        match token {
            SERVER => {
                let stream = self.unix_listener.accept().unwrap().unwrap();
                match self.conns.insert_with(|token| {
                    println!("registering {:?} with event loop", token);
                    Connection::new(stream, token)
                }) {
                    Some(token) => {
                        // If we successfully insert, then register our connection.
                        let conn = &mut self.conns[token];
                        event_loop.register_opt(
                            &conn.stream,
                            conn.token,
                            mio::EventSet::readable(),
                            mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
                    },
                    None => {
                        // If we fail to insert, `conn` will go out of scope and be dropped.
                        panic!("Failed to insert connection into slab");
                    }
                };
            },
            client_token => {
                if events.is_hup() {
                    self.conns.remove(client_token);
                } else if events.is_readable() {
                    let mut vec = Vec::new();
                    {
                        let conn = &mut self.conns[token];
                        conn.stream.read_message(&mut vec);
                        event_loop.reregister(
                            &conn.stream,
                            conn.token,
                            mio::EventSet::readable(),
                            mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
                    }
                    let s = String::from_utf8(vec).unwrap();
                    let value: json::Value = json::from_str(&s).unwrap();
                    println!("#sirver value: {:#?}", value);
                    let b = value.find("type").and_then(|o| o.as_string()).unwrap();
                    if value.find("type").and_then(|o| o.as_string()) == Some("call") {
                        let name = value.find("function")
                            .and_then(|o| o.as_string()).unwrap();
                        let function = self.functions.get(name).unwrap();
                        function.call(value.find("args").unwrap_or(&json::builder::ObjectBuilder::new().unwrap()));
                    };

                    println!("#sirver s: {:#?}", s);
                }
            }
        }
    }
}

pub fn run_supreme_server() {
    // TODO(sirver): grep for unwrap and remove
    let server = UnixListener::bind("/tmp/s.socket").unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER);

    let mut s_server = SupremeServer {
        buffers: Vec::new(),
        unix_listener: server,
        conns: mio::util::Slab::new_starting_at(mio::Token(1), 1024),
        functions: HashMap::new(),
    };
    plugin_core::register(&mut s_server);

    event_loop.run(&mut s_server).unwrap();
}
