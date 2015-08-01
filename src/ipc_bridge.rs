use mio::unix::{UnixListener, UnixStream};
use mio;
use std::path::Path;
// NOCOM(#sirver): rename IpcRead and IpcWrite to ipc::Read
use super::ipc::{self, IpcRead, IpcWrite};
use super::plugin::remote::{RemotePlugin};
use super::plugin::{RemotePluginId, PluginId, FunctionCallContext};
use super::server;

struct Connection {
    stream: UnixStream,
    remote_plugin_id: RemotePluginId,
}

pub struct IpcBridge {
    unix_listener: UnixListener,
    connections: mio::util::Slab<Connection>,
    commands: server::CommandSender,
    next_serial: u64,
}

const SERVER_TOKEN: mio::Token = mio::Token(0);

impl IpcBridge {
    pub fn new(event_loop: &mut mio::EventLoop<Self>, socket_name: &Path, server_commands: server::CommandSender) -> Self {
        let server = UnixListener::bind(socket_name).unwrap();
        event_loop.register(&server, SERVER_TOKEN).unwrap();
        IpcBridge {
            unix_listener: server,
            connections: mio::util::Slab::new_starting_at(mio::Token(1), 1024),
            commands: server_commands,
            next_serial: 1,
        }
    }
}

pub enum Command {
    Quit,
    SendData(RemotePluginId, ipc::Message),
}

impl mio::Handler for IpcBridge {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::SendData(receiver, message) => {
                // NOCOM(#sirver): what if that is no longer valid?
                let conn = &mut self.connections[receiver.token];
                if conn.remote_plugin_id == receiver {
                    if let Some(err) = conn.stream.write_message(&message).err() {
                        println!("Could not send message to {:?}: {}", receiver, err);
                    }
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
                    let remote_plugin_id = RemotePluginId {
                        serial: serial,
                        token: token,
                    };
                    let plugin = RemotePlugin {
                        id: PluginId::Remote(remote_plugin_id),
                        ipc_bridge_commands: event_loop.channel(),
                    };
                    let connection = Connection {
                        stream: stream,
                        remote_plugin_id: remote_plugin_id,
                    };
                    commands.send(server::Command::PluginConnected(Box::new(plugin))).unwrap();
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
                    let connection = self.connections.remove(client_token).unwrap();
                    self.commands.send(
                        server::Command::PluginDisconnected(
                            PluginId::Remote(connection.remote_plugin_id))).unwrap();
                    return;
                }

                if events.is_readable() {
                    let conn = &mut self.connections[token];
                    // NOCOM(#sirver): should disconnect instead of crashing.
                    let message = conn.stream.read_message().expect("Could not read_message");;
                    match message {
                        ipc::Message::RpcCall(rpc_call) => {
                            let call_context = FunctionCallContext {
                                rpc_call: rpc_call,
                                commands: self.commands.clone(),
                                caller: PluginId::Remote(conn.remote_plugin_id),
                            };
                            self.commands.send(server::Command::CallFunction(call_context)).unwrap();

                            // NOCOM(#sirver): need to keep track of how called this and how
                        },
                        ipc::Message::RpcReply(_) => {
                            // NOCOM(#sirver): should not be broadcasted - only the caller is
                            // interested in that.
                            self.commands.send(server::Command::Broadcast(message)).unwrap();
                        }
                        _ => panic!("Client send unexpected commands."),
                    }
                }
            }
        }
    }
}
