use mio::unix::{UnixListener, UnixStream};
use mio;
use serde::json;
use super::ipc::{IpcRead, IpcWrite};
use super::plugin::{RemotePluginId, PluginId, FunctionCallContext};
use super::plugin::remote::{RemotePlugin};
use super::server::{Command, CommandSender};

// NOCOM(#sirver): ClientConnection?
struct Connection {
    stream: UnixStream,
    remote_plugin_id: RemotePluginId,
}

pub struct IpcBridge {
    unix_listener: UnixListener,
    connections: mio::util::Slab<Connection>,
    commands: CommandSender,
    next_serial: u64,
}

const SERVER_TOKEN: mio::Token = mio::Token(0);

impl IpcBridge {
    pub fn new(event_loop: &mut mio::EventLoop<Self>, uds_path: &str, server_commands: CommandSender) -> Self {
        let server = UnixListener::bind(uds_path).unwrap();
        event_loop.register(&server, SERVER_TOKEN).unwrap();
        IpcBridge {
            unix_listener: server,
            connections: mio::util::Slab::new_starting_at(mio::Token(1), 1024),
            commands: server_commands,
            next_serial: 1,
        }
    }
}

pub enum HandlerMessage {
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
            SERVER_TOKEN => {
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
