// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::ipc;
use ::server;
use ::{ErrorKind, Error};
use mio::tcp::{TcpListener, TcpStream};
use mio::unix::{UnixListener, UnixStream};
use mio;
use std::net;
use std::path::Path;
use std::str::FromStr;
use threadpool::ThreadPool;
use std::io;

// Number of threads to use for handling IO.
const NUM_THREADS: usize = 4;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct ClientId {
    pub serial: u64,
    pub token: mio::Token,
}

// We abstract over unix and TCP connections. Since receiver and sender both get a copy of the
// socket, we need to clone them. Since we store them in slab (which means the trait cannot be
// sized), we have to return boxes too.
trait MioStream: Send + io::Read + io::Write + mio::Evented {
    fn try_clone(&self) -> io::Result<Box<MioStream>>;
}

impl MioStream for UnixStream {
    fn try_clone(&self) -> io::Result<Box<MioStream>> {
        UnixStream::try_clone(&self).map(|v| Box::new(v) as Box<MioStream>)
    }
}

impl MioStream for TcpStream {
    fn try_clone(&self) -> io::Result<Box<MioStream>> {
        TcpStream::try_clone(&self).map(|v| Box::new(v) as Box<MioStream>)
    }
}


struct Connection<T: io::Read + io::Write> {
    // NOCOM(#sirver): messy design
    reader: Option<ipc::Reader<T>>,
    writer: ipc::Writer<T>,
    client_id: ClientId,
}

pub struct IpcBridge {
    unix_listener: UnixListener,
    tcp_listeners: Vec<TcpListener>,
    connections: mio::util::Slab<Connection<Box<MioStream>>>,
    commands: server::CommandSender,
    first_client_token: usize,
    next_serial: u64,
    thread_pool: ThreadPool,
}

const UNIX_LISTENER: mio::Token = mio::Token(0);

impl IpcBridge {
    pub fn new(event_loop: &mut mio::EventLoop<Self>,
               socket_name: &Path,
               tcp_addresses: &Vec<String>,
               server_commands: server::CommandSender) -> Self {
        let unix_listener = UnixListener::bind(socket_name).unwrap();
        event_loop.register(
            &unix_listener,
            UNIX_LISTENER,
            mio::EventSet::readable(),
            mio::PollOpt::edge()).unwrap();

        let mut first_client_token = 1;
        let tcp_listeners: Vec<_> =
            tcp_addresses.iter()
            .map(|addr| {
                let addr = net::SocketAddr::from_str(addr).unwrap();
                let server = TcpListener::bind(&addr).unwrap();
                event_loop.register(
                    &server,
                    mio::Token(first_client_token),
                    mio::EventSet::readable(),
                    mio::PollOpt::edge()).unwrap();
                first_client_token += 1;
                server
            }).collect();

        IpcBridge {
            unix_listener: unix_listener,
            tcp_listeners: tcp_listeners,
            first_client_token: first_client_token,
            connections: mio::util::Slab::new_starting_at(mio::Token(first_client_token), 1024),
            commands: server_commands,
            next_serial: 1,
            thread_pool: ThreadPool::new(NUM_THREADS),
        }
    }

    fn new_client<T: MioStream + 'static>(&mut self, event_loop: &mut mio::EventLoop<Self>, stream: Box<T>) {
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
                event_loop.register(
                    &*conn.reader.as_ref().unwrap().socket,
                    conn.client_id.token,
                    mio::EventSet::readable(),
                    mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
            },
            None => {
                // If we fail to insert, `conn` will go out of scope and be dropped.
                panic!("Failed to insert connection into slab");
            }
        };
    }
}

pub enum Command {
    Quit,
    SendData(ClientId, ipc::Message),
    ReRegisterForReading(mio::Token, ipc::Reader<Box<MioStream>>),
}

impl mio::Handler for IpcBridge {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::SendData(receiver, message) => {
                let result = self.connections.get_mut(receiver.token)
                    .ok_or(Error::new(ErrorKind::Disconnected))
                    .and_then(|conn| {
                        if conn.client_id != receiver {
                            Err(Error::new(ErrorKind::Disconnected))
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
                    &*conn.reader.as_ref().unwrap().socket,
                    token,
                    mio::EventSet::readable(),
                    mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
            },
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            UNIX_LISTENER => {
                // Unix domain socket connection.
                let stream = self.unix_listener.accept().unwrap().unwrap();
                self.new_client(event_loop, Box::new(stream));

            },
            mio::Token(some_token) if some_token < self.first_client_token => {
                // TCP connection
                let stream = self.tcp_listeners[some_token - 1].accept().unwrap().unwrap();
                self.new_client(event_loop, Box::new(stream));
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
