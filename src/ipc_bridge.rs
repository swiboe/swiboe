// NOCOM(#sirver): rename IpcRead and IpcWrite to ipc::Read

use mio::unix::{UnixListener, UnixStream};
use mio;
use std::path::Path;
use super::ipc::{self, IpcRead, IpcWrite};
use super::server;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct ClientId {
    pub serial: u64,
    pub token: mio::Token,
}

struct Connection {
    stream: UnixStream,
    client_id: ClientId,
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
        event_loop.register_opt(
            &server,
            SERVER_TOKEN,
            mio::EventSet::readable(),
            mio::PollOpt::level()).unwrap();
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
    SendData(ClientId, ipc::Message),
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
                if conn.client_id == receiver {
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
                    let client_id = ClientId {
                        serial: serial,
                        token: token,
                    };
                    let connection = Connection {
                        stream: stream,
                        client_id: client_id,
                    };
                    commands.send(server::Command::ClientConnected(client_id)).unwrap();
                    connection
                }) {
                    Some(token) => {
                        // If we successfully insert, then register our connection.
                        let conn = &mut self.connections[token];
                        event_loop.register_opt(
                            &conn.stream,
                            conn.client_id.token,
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
                        server::Command::ClientDisconnected(connection.client_id)).unwrap();
                } else if events.is_readable() {
                    let conn = &mut self.connections[token];
                    // NOCOM(#sirver): should disconnect instead of crashing.
                    let message = conn.stream.read_message().expect("Could not read_message");;
                    match message {
                        ipc::Message::RpcCall(rpc_call) => {
                            self.commands.send(server::Command::RpcCall(conn.client_id, rpc_call)).unwrap();
                        },
                        ipc::Message::RpcResponse(rpc_response) => {
                            self.commands.send(server::Command::RpcResponse(rpc_response)).unwrap();
                        }
                    }
                }
            }
        }
    }
}
