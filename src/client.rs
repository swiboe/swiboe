#![allow(deprecated)]

use serde::json;

// NOCOM(#sirver): use a custom enum for error codes even in json.

use mio::unix::UnixStream;
use mio;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use super::ipc::{self, IpcWrite, IpcRead};
use super::plugin_core::RegisterFunctionArgs;
use super::{Error, Result, ErrorKind};
use uuid::Uuid;

const CLIENT: mio::Token = mio::Token(1);

pub trait RemoteProcedure {
    fn call(&mut self, client: &Client, args: json::Value) -> ipc::RpcResultKind;
}

// NOCOM(#sirver): why is that pub?
pub struct Rpc {
    // NOCOM(#sirver): something more structured?
    pub values: mpsc::Receiver<ipc::RpcReply>,
}

impl Rpc {
    fn new(values: mpsc::Receiver<ipc::RpcReply>) -> Self {
        // NOCOM(#sirver): implement drop so that we can cancel an RPC.
        Rpc {
            values: values,
        }
    }

    // NOCOM(#sirver): timeout?
    fn recv(&self) -> Result<ipc::RpcReply> {
        Ok(try!(self.values.recv()))
    }

    pub fn wait(self) -> Result<ipc::RpcResultKind> {
        loop {
            let rpc_reply = try!(self.recv());
            if rpc_reply.state == ipc::RpcState::Done {
                return Ok(rpc_reply.result);
            }
            unimplemented!();
            // NOCOM(#sirver): put data into queue.
        }
    }
}

pub struct Client<'a> {
    values: mpsc::Receiver<json::Value>,
    network_commands: mio::Sender<Command>,
    remote_procedures: HashMap<String, Box<RemoteProcedure>>,
    _event_loop_thread_guard: thread::JoinGuard<'a, ()>,
}

pub enum Command {
    Quit,
    Send(ipc::Message),
    Call(String, mpsc::Sender<ipc::RpcReply>),
}

// NOCOM(#sirver): bad name
struct Handler {
    stream: UnixStream,
    values: mpsc::Sender<json::Value>,
    running_function_calls: HashMap<String, mpsc::Sender<ipc::RpcReply>>,
}

impl mio::Handler for Handler {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::Send(message) => {
                if let Err(err) = self.stream.write_message(&message) {
                    println!("Shutting down, since sending failed: {}", err);
                    event_loop.channel().send(Command::Quit).unwrap();
                }
            },
            Command::Call(context, tx) => {
                self.running_function_calls.insert(context, tx);
            }
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            CLIENT => {
                if events.is_hup() {
                    event_loop.channel().send(Command::Quit).unwrap();
                    return;
                }

                if events.is_readable() {
                    let message = match self.stream.read_message() {
                        Ok(message) => message,
                        Err(err) => {
                            println!("Shutting down, since receiving failed: {}", err);
                            event_loop.channel().send(Command::Quit).unwrap();
                            return;
                        }
                    };

                    match message {
                        ipc::Message::RpcData(rpc_data) => {
                            // This will quietly drop any updates on functions that we no longer
                            // know/care about.
                            self.running_function_calls
                                .get(&rpc_data.context)
                                .map(|channel| {
                                    channel.send(rpc_data).unwrap();
                                });
                        },
                        ipc::Message::Broadcast(data) => {
                            self.values.send(data).unwrap();
                        },
                        _ => panic!("Server send unexpected commands."),
                    }
                }
            },
            client_token => panic!("Unexpected token: {:?}", client_token),
        }
    }
}

impl<'a> Client<'a> {
    // NOCOM(#sirver): socket_name should be a path
    pub fn connect(socket_name: &str) -> Self {
        let stream =
            UnixStream::connect(socket_name).unwrap();

        let mut event_loop = mio::EventLoop::new().unwrap();
        event_loop.register_opt(
                            &stream,
                            CLIENT,
                            mio::EventSet::readable(),
                            mio::PollOpt::level()).unwrap();

        let (client_tx, values) = mpsc::channel();
        let network_commands = event_loop.channel();
        let event_loop_thread_guard = thread::scoped(move || {
            event_loop.run(&mut Handler {
                stream: stream,
                values: client_tx,
                running_function_calls: HashMap::new(),
            }).unwrap();
        });

        let client = Client {
            values: values,
            network_commands: network_commands,
            remote_procedures: HashMap::new(),
            _event_loop_thread_guard: event_loop_thread_guard,
        };
        client
    }

    pub fn write(&self, message: ipc::Message) {
        self.network_commands.send(Command::Send(message)).unwrap();
    }

    pub fn recv(&self) -> Result<json::Value> {
        match self.values.recv() {
            Ok(value) => Ok(value),
            Err(err) => Err(Error::new(ErrorKind::Disconnected(err))),
        }
    }

    // NOCOM(#sirver): Return a future? How about streaming functions?
    pub fn call(&self, function: &str, args: &json::Value) -> Rpc {
        let context = Uuid::new_v4().to_hyphenated_string();

        let message = ipc::Message::RpcCall {
            function: function.into(),
            context: context.clone(),
            args: args.clone(),
        };

        let (tx, rx) = mpsc::channel();
        self.network_commands.send(Command::Call(context, tx)).unwrap();

        self.write(message);
        Rpc::new(rx)
    }

    pub fn register_function(&mut self, name: &str, remote_procedure: Box<RemoteProcedure>) {
        // NOCOM(#sirver): what happens when this is already inserted? crash probably
        // NOCOM(#sirver): rethink 'register_function' maybe, register_rpc
        let rpc = self.call("core.register_function", &json::to_value(&RegisterFunctionArgs {
            name: name.into(),
        }));
        let success = rpc.wait().unwrap();
        // NOCOM(#sirver): report failure.

        self.remote_procedures.insert(name.into(), remote_procedure);
    }
}

impl<'a> Drop for Client<'a> {
    fn drop(&mut self) {
        // The event loop is already shut down if the server disconnected us. Then this send will
        // fail, which is fine to be ignored in that case.
        let _ = self.network_commands.send(Command::Quit);
    }
}
