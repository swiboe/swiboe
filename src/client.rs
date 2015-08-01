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

pub trait RemoteProcedure: Send {
    fn call(&mut self, args: json::Value) -> ipc::RpcResultKind;
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
    event_loop_sender: mio::Sender<Command>,
    _event_loop_thread_guard: thread::JoinGuard<'a, ()>,

    function_thread_sender: mpsc::Sender<FunctionThreadCommand>,
    _function_thread_guard: thread::JoinGuard<'a, ()>,
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
    function_thread_sender: mpsc::Sender<FunctionThreadCommand>,
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
            },
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
                        ipc::Message::RpcCall { function, context, args } => {
                            // NOCOM(#sirver): can a : a below be skipped?
                            let command = FunctionThreadCommand::Call(ipc::RpcCall {
                                function: function,
                                context: context,
                                args: args
                            });
                            self.function_thread_sender.send(command).unwrap();
                        },
                        ipc::Message::Broadcast(data) => {
                            self.values.send(data).unwrap();
                        },
                    }
                }
            },
            client_token => panic!("Unexpected token: {:?}", client_token),
        }
    }
}

// NOCOM(#sirver): urg... that is the real client.
pub struct ClientHandle {
    event_loop_sender: mio::Sender<Command>,
    function_thread_sender: mpsc::Sender<FunctionThreadCommand>,
}

impl ClientHandle {
    pub fn write(&self, message: ipc::Message) {
        self.event_loop_sender.send(Command::Send(message)).unwrap();
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
        self.event_loop_sender.send(Command::Call(context, tx)).unwrap();

        self.write(message);
        Rpc::new(rx)
    }
}

enum FunctionThreadCommand {
    Quit,
    RegisterFunction(String, Box<RemoteProcedure>),
    Call(ipc::RpcCall),
}

struct FunctionThread {
    remote_procedures: HashMap<String, Box<RemoteProcedure>>,
    commands: mpsc::Receiver<FunctionThreadCommand>,
    event_loop_sender: mio::Sender<Command>,
}

impl FunctionThread {
    pub fn spin_forever(mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                FunctionThreadCommand::Quit => break,
                FunctionThreadCommand::RegisterFunction(name, remote_procedures) => {
                    self.remote_procedures.insert(name, remote_procedures);
                },
                FunctionThreadCommand::Call(rpc_call) => {
                    if let Some(function) = self.remote_procedures.get_mut(&rpc_call.function) {
                        // NOCOM(#sirver): result value?
                        let result = function.call(rpc_call.args);
                        self.event_loop_sender.send(Command::Send(
                            ipc::Message::RpcData(ipc::RpcReply {
                                context: rpc_call.context,
                                // NOCOM(#sirver): what about streaming rpcs?
                                state: ipc::RpcState::Done,
                                result: result,
                            }))).unwrap();
                    }
                    // NOCOM(#sirver): return an error - though if that has happened the
                    // server messed up too.

                    // NOCOM(#sirver): implement
                }
            }
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
        let event_loop_sender = event_loop.channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        // NOCOM(#sirver): the Handler could maybe dispatch all commands between threads?
        let event_loop_function_thread_sender = commands_tx.clone();
        let event_loop_thread_guard = thread::scoped(move || {
            event_loop.run(&mut Handler {
                stream: stream,
                values: client_tx,
                running_function_calls: HashMap::new(),
                function_thread_sender: event_loop_function_thread_sender,
            }).unwrap();
        });

        let function_thread_event_loop_sender = event_loop_sender.clone();
        let function_thread_guard = thread::scoped(move || {
            let thread = FunctionThread {
                remote_procedures: HashMap::new(),
                commands: commands_rx,
                event_loop_sender: function_thread_event_loop_sender,
            };
            thread.spin_forever();
        });

        let client = Client {
            values: values,
            event_loop_sender: event_loop_sender,
            _event_loop_thread_guard: event_loop_thread_guard,

            function_thread_sender: commands_tx,
            _function_thread_guard: function_thread_guard,
        };
        client
    }

    pub fn client_handle(&self) -> ClientHandle {
        ClientHandle {
            event_loop_sender: self.event_loop_sender.clone(),
            function_thread_sender: self.function_thread_sender.clone(),
        }
    }

    pub fn recv(&self) -> Result<json::Value> {
        match self.values.recv() {
            Ok(value) => Ok(value),
            Err(err) => Err(Error::new(ErrorKind::Disconnected(err))),
        }
    }

    // NOCOM(#sirver): now, that seems really stupid..
    pub fn call(&self, function: &str, args: &json::Value) -> Rpc {
        self.client_handle().call(function, args)
    }

    pub fn register_function(&self, name: &str, remote_procedure: Box<RemoteProcedure>) {
        // NOCOM(#sirver): what happens when this is already inserted? crash probably
        // NOCOM(#sirver): rethink 'register_function' maybe, register_rpc
        let rpc = self.call("core.register_function", &json::to_value(&RegisterFunctionArgs {
            name: name.into(),
        }));
        let success = rpc.wait().unwrap();
        // NOCOM(#sirver): report failure.

        self.function_thread_sender.send(FunctionThreadCommand::RegisterFunction(
                name.into(), remote_procedure)).unwrap();
    }
}

impl<'a> Drop for Client<'a> {
    fn drop(&mut self) {
        // The event loop is already shut down if the server disconnected us. Then this send will
        // fail, which is fine to be ignored in that case.
        let _ = self.event_loop_sender.send(Command::Quit);
        let _ = self.function_thread_sender.send(FunctionThreadCommand::Quit);
    }
}
