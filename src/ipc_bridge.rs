use ::ipc;
use ::server;
use ::{ErrorKind, Error};
use mio::unix::{UnixListener, UnixStream};
use mio;
use std::path::Path;
use threadpool::ThreadPool;

// Number of threads to use for handling IO.
const NUM_THREADS: usize = 4;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct ClientId {
    pub serial: u64,
    pub token: mio::Token,
}

struct Connection {
    // NOCOM(#sirver): messy design
    reader: Option<ipc::Reader<UnixStream>>,
    writer: ipc::Writer<UnixStream>,
    client_id: ClientId,
}

pub struct IpcBridge {
    unix_listener: UnixListener,
    connections: mio::util::Slab<Connection>,
    commands: server::CommandSender,
    next_serial: u64,
    thread_pool: ThreadPool,
}

const SERVER_TOKEN: mio::Token = mio::Token(0);

impl IpcBridge {
    pub fn new(event_loop: &mut mio::EventLoop<Self>, socket_name: &Path,
               server_commands: server::CommandSender) -> Self {
        let server = UnixListener::bind(socket_name).unwrap();
        event_loop.register_opt(
            &server,
            SERVER_TOKEN,
            mio::EventSet::readable(),
            mio::PollOpt::edge()).unwrap();
        IpcBridge {
            unix_listener: server,
            connections: mio::util::Slab::new_starting_at(mio::Token(1), 1024),
            commands: server_commands,
            next_serial: 1,
            thread_pool: ThreadPool::new(NUM_THREADS),
        }
    }
}

pub enum Command {
    Quit,
    SendData(ClientId, ipc::Message),
    ReRegisterForReading(mio::Token, ipc::Reader<UnixStream>),
}

impl mio::Handler for IpcBridge {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::SendData(receiver, message) => {
                let result = self.connections.get_mut(receiver.token)
                    .ok_or(Error::new(ErrorKind::ClientDisconnected))
                    .and_then(|conn| {
                        if conn.client_id != receiver {
                            Err(Error::new(ErrorKind::ClientDisconnected))
                        } else {
                            // println!("{:?}: Server -> {:?}: {:#?}", time::precise_time_ns(),
                                // receiver, message);
                            // NOCOM(#sirver): super dangerous - the reader might be handed out.
                            conn.writer.write_message(&message)
                        }
                    });
                if let Err(err) = result {
                    self.commands.send(server::Command::SendDataFailed(receiver, message, err)).expect("SendFailed");
                }
            }
            Command::ReRegisterForReading(token, reader) => {
                // NOCOM(#sirver): I think this is dangerous.
                let conn = &mut self.connections[token];
                conn.reader = Some(reader);
                event_loop.reregister(
                    &conn.reader.as_ref().unwrap().socket,
                    token,
                    mio::EventSet::readable(),
                    mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
            },
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
                        writer: ipc::Writer::new(stream.try_clone().unwrap()),
                        reader: Some(ipc::Reader::new(stream)),
                        client_id: client_id,
                    };
                    commands.send(server::Command::ClientConnected(client_id)).expect("ClientConnected");
                    connection
                }) {
                    Some(token) => {
                        // If we successfully insert, then register our connection.
                        let conn = &mut self.connections[token];
                        event_loop.register_opt(
                            &conn.reader.as_ref().unwrap().socket,
                            conn.client_id.token,
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
                    let connection = self.connections.remove(client_token).unwrap();
                    self.commands.send(
                        server::Command::ClientDisconnected(connection.client_id)).expect("ClientDisconnected");
                } else if events.is_readable() {
                    let conn = &mut self.connections[token];
                    let mut reader = conn.reader.take().unwrap();
                    let commands = self.commands.clone();
                    let client_id = conn.client_id;
                    let event_loop_sender = event_loop.channel();
                    self.thread_pool.execute(move || {
                        loop {
                            match reader.read_message() {
                                // NOCOM(#sirver): should disconnect instead of panic.
                                Err(err) => panic!("Error while reading: {}", err),
                                Ok(None) => break,
                                Ok(Some(message)) => {
                                    // println!("{:?}: {:?} -> Server: {:#?}", time::precise_time_ns(),
                                    // client_id, message);
                                    match message {
                                        // NOCOM(#sirver): pack them together in one message?
                                        ipc::Message::RpcCall(rpc_call) => {
                                            commands.send(server::Command::RpcCall(
                                                    client_id, rpc_call)).expect("RpcCall");
                                        },
                                        ipc::Message::RpcResponse(rpc_response) => {
                                            commands.send(server::Command::RpcResponse(rpc_response)).expect("RpcResponse");
                                        },
                                        ipc::Message::RpcCancel(rpc_cancel) => {
                                            commands.send(server::Command::RpcCancel(rpc_cancel)).expect("RpcCancel");
                                        },
                                    }
                                }
                            }
                        }
                        event_loop_sender.send(Command::ReRegisterForReading(token, reader)).expect("Command::ReRegisterForReading");
                    });
                }
            }
        }
    }
}
