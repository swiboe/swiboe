#![allow(deprecated)]

use ::Result;
use ::ipc;
use ::plugin_core::NewRpcRequest;
use ::rpc;
use mio::unix::UnixStream;
use mio;
use serde::{json, Serialize, Deserialize};
use std::collections::HashMap;
use std::path;
use std::result;
use std::sync::mpsc;
use std::thread;
use uuid::Uuid;

const CLIENT: mio::Token = mio::Token(1);

pub trait RemoteProcedure: Send {
    fn priority(&self) -> u16 { u16::max_value() }
    fn call(&mut self, context: RpcServerContext, args: json::Value);
}

pub struct RpcClientContext {
    context: String,
    values: mpsc::Receiver<rpc::Response>,
    result: Option<rpc::Result>,
    event_loop_sender: mio::Sender<EventLoopThreadCommand>,
}

#[derive(Debug)]
pub enum RpcClientContextError {
    Disconnected,
    InvalidOrUnexpectedReply(json::Error),
}

// NOCOM(#sirver): impl error::Error for RpcClientContextError?

impl From<mio::NotifyError<EventLoopThreadCommand>> for RpcClientContextError {
    fn from(_: mio::NotifyError<EventLoopThreadCommand>) -> Self {
        RpcClientContextError::Disconnected
    }
}

impl From<mpsc::RecvError> for RpcClientContextError {
    fn from(_: mpsc::RecvError) -> Self {
        RpcClientContextError::Disconnected
    }
}

impl From<json::error::Error> for RpcClientContextError {
     fn from(error: json::error::Error) -> Self {
         RpcClientContextError::InvalidOrUnexpectedReply(error)
     }
}

pub type RpcClientContextResult<T> = result::Result<T, RpcClientContextError>;

impl RpcClientContext {
    fn new<T: Serialize>(event_loop_sender: &mio::Sender<EventLoopThreadCommand>,
                         function: &str,
                         args: &T) -> result::Result<Self, mio::NotifyError<EventLoopThreadCommand>> {
        let args = json::to_value(&args);
        let context = Uuid::new_v4().to_hyphenated_string();
        let message = ipc::Message::RpcCall(rpc::Call {
            function: function.into(),
            context: context.clone(),
            args: args,
        });

        let (tx, rx) = mpsc::channel();
        try!(event_loop_sender.send(EventLoopThreadCommand::Call(context.clone(), tx, message)));
        // NOCOM(#sirver): implement drop so that we can cancel an RPC.
        Ok(RpcClientContext {
            values: rx,
            event_loop_sender: event_loop_sender.clone(),
            context: context,
            result: None,
        })
    }

    // NOCOM(#sirver): timeout?
    pub fn recv(&mut self) -> RpcClientContextResult<Option<json::Value>> {
        if self.result.is_some() {
            return Ok(None);
        }

        let rpc_response = try!(self.values.recv());
        match rpc_response.kind {
            rpc::ResponseKind::Partial(value) => Ok(Some(value)),
            rpc::ResponseKind::Last(result) => {
                self.result = Some(result);
                Ok(None)
            },
        }
    }

    pub fn wait(&mut self) -> RpcClientContextResult<rpc::Result> {
        while let Some(_) = try!(self.recv()) {
        }
        Ok(self.result.take().unwrap())
    }

    // NOCOM(#sirver): figure out error handling for clients, not use Server error?
    pub fn wait_for<T: Deserialize>(&mut self) -> RpcClientContextResult<T> {
        match try!(self.wait()) {
            rpc::Result::Ok(value) => Ok(try!(json::from_value(value))),
            rpc::Result::Err(err) => panic!("#sirver err: {:#?}", err),
            // NOCOM(#sirver): probably should ignore other errors.
            other => panic!("#sirver other: {:#?}", other),
        }
    }

    pub fn cancel(self) -> RpcClientContextResult<()> {
        let msg = ipc::Message::RpcCancel(rpc::Cancel {
            context: self.context.clone(),
        });
        try!(self.event_loop_sender.send(EventLoopThreadCommand::Send(msg)));
        Ok(())
    }
}

pub enum EventLoopThreadCommand {
    Quit,
    Send(ipc::Message),
    Call(String, mpsc::Sender<rpc::Response>, ipc::Message),
}

// NOCOM(#sirver): bad name
struct Handler<'a> {
    stream: ipc::Stream<UnixStream>,
    running_function_calls: HashMap<String, mpsc::Sender<rpc::Response>>,
    function_thread_sender: mpsc::Sender<FunctionThreadCommand<'a>>,
}

impl<'a> Handler<'a> {
    fn send(&mut self, event_loop: &mut mio::EventLoop<Self>, message: &ipc::Message) {
        // println!("{:?}: Client -> Server {:?}", time::precise_time_ns(), message);
        if let Err(err) = self.stream.write_message(&message) {
            println!("Shutting down, since sending failed: {:?}", err);
            event_loop.channel().send(EventLoopThreadCommand::Quit).expect("Quit");
        }
    }
}


impl<'a> mio::Handler for Handler<'a> {
    type Timeout = ();
    type Message = EventLoopThreadCommand;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: EventLoopThreadCommand) {
        match command {
            EventLoopThreadCommand::Quit => event_loop.shutdown(),
            EventLoopThreadCommand::Send(message) => self.send(event_loop, &message),
            EventLoopThreadCommand::Call(context, tx, message) => {
                self.running_function_calls.insert(context, tx);
                self.send(event_loop, &message)
            },
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            CLIENT => {
                if events.is_hup() {
                    event_loop.channel().send(EventLoopThreadCommand::Quit).expect("EventLoopThreadCommand::Quit");
                    return;
                }

                if events.is_readable() {
                    loop {
                        let message;
                        match self.stream.read_message() {
                            Err(err) => {
                                println!("Shutting down, since receiving failed: {:?}", err);
                                event_loop.channel().send(EventLoopThreadCommand::Quit).expect("EventLoopThreadCommand::Quit");
                                return;
                            }
                            Ok(None) => break,
                            Ok(Some(msg)) => message = msg,
                        };

                        match message {
                            ipc::Message::RpcResponse(rpc_data) => {
                                // NOCOM(#sirver): if this is a streaming RPC, we should cancel the
                                // RPC.
                                // This will quietly drop any updates on functions that we no longer
                                // know/care about.
                                self.running_function_calls
                                    .get(&rpc_data.context)
                                    .map(|channel| {
                                        // The other side of this channel might not exist anymore - we
                                        // might have dropped the RPC already. Just ignore it.
                                        let _ = channel.send(rpc_data);
                                    });
                            },
                            ipc::Message::RpcCall(rpc_call) => {
                                let command = FunctionThreadCommand::Call(rpc_call);
                                self.function_thread_sender.send(command).expect("FunctionThreadCommand::Call");
                            },
                            ipc::Message::RpcCancel(rpc_cancel) => {
                                let command = FunctionThreadCommand::Cancel(rpc_cancel);
                                self.function_thread_sender.send(command).expect("FunctionThreadCommand::Cancel");
                            }
                        }
                    }
                    event_loop.reregister(
                        &self.stream.socket,
                        CLIENT,
                        mio::EventSet::readable(),
                        mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
                }
            },
            client_token => panic!("Unexpected token: {:?}", client_token),
        }
    }
}

enum FunctionThreadCommand<'a> {
    Quit,
    NewRpc(String, Box<RemoteProcedure + 'a>),
    Call(rpc::Call),
    Cancel(rpc::Cancel),
}

enum RpcServerContextCommand {
    Cancel,
}

struct RunningRpc {
    commands: mpsc::Sender<RpcServerContextCommand>,
}

impl RunningRpc {
    fn new(commands: mpsc::Sender<RpcServerContextCommand>) -> Self {
        RunningRpc {
            commands: commands,
        }
    }
}

struct FunctionThread<'a> {
    remote_procedures: HashMap<String, Box<RemoteProcedure + 'a>>,
    commands: mpsc::Receiver<FunctionThreadCommand<'a>>,
    event_loop_sender: mio::Sender<EventLoopThreadCommand>,
    running_rpc_calls: HashMap<String, RunningRpc>,
}

impl<'a> FunctionThread<'a> {
    fn spin_forever(&mut self) {
        while let Ok(command) = self.commands.recv() {
            match command {
                FunctionThreadCommand::Quit => break,
                FunctionThreadCommand::NewRpc(name, remote_procedures) => {
                    self.remote_procedures.insert(name, remote_procedures);
                },
                FunctionThreadCommand::Call(rpc_call) => {
                    if let Some(function) = self.remote_procedures.get_mut(&rpc_call.function) {
                        let (tx, rx) = mpsc::channel();
                        function.call(RpcServerContext::new(
                            rpc_call.context.clone(), rx, self.event_loop_sender.clone()), rpc_call.args);
                        self.running_rpc_calls.insert(rpc_call.context.clone(), RunningRpc::new(tx));
                    }
                    // NOCOM(#sirver): return an error - though if that has happened the
                    // server messed up too.
                }
                FunctionThreadCommand::Cancel(rpc_cancel) => {
                    println!("#sirver rpc_cancel: {:#?}", rpc_cancel);
                    // NOCOM(#sirver): on drop, the rpcservercontext must delete the entry.
                    if let Some(function) = self.running_rpc_calls.remove(&rpc_cancel.context) {
                        // The function might be dead already, so we ignore errors.
                        let _ = function.commands.send(RpcServerContextCommand::Cancel);
                    }
                }
            }
        }
    }
}

pub struct Client<'a> {
    event_loop_sender: mio::Sender<EventLoopThreadCommand>,
    _function_thread_join_guard: thread::JoinGuard<'a, ()>,

    function_thread_sender: mpsc::Sender<FunctionThreadCommand<'a>>,
}

impl<'a> Client<'a> {
    pub fn connect(socket_name: &path::Path) -> Self {
        let stream = UnixStream::connect(&socket_name).unwrap();

        let mut event_loop = mio::EventLoop::<Handler>::new().unwrap();
        let event_loop_sender = event_loop.channel();

        let (commands_tx, commands_rx) = mpsc::channel();
        let mut handler = Handler {
            stream: ipc::Stream::new(stream),
            running_function_calls: HashMap::new(),
            function_thread_sender: commands_tx.clone(),
        };
        event_loop.register_opt(
            &handler.stream.socket, CLIENT, mio::EventSet::readable(), mio::PollOpt::edge() |
            mio::PollOpt::oneshot()).unwrap();
        let event_loop_thread = thread::scoped(move || {
            event_loop.run(&mut handler).unwrap();
        });

        Client {
            event_loop_sender: event_loop_sender.clone(),
            function_thread_sender: commands_tx,
            _function_thread_join_guard: thread::scoped(move || {
                let mut thread = FunctionThread {
                    remote_procedures: HashMap::new(),
                    running_rpc_calls: HashMap::new(),
                    commands: commands_rx,
                    event_loop_sender: event_loop_sender,
                };
                thread.spin_forever();

                // If the remote side has disconnected us, the event_loop will already be destroyed and
                // this send will fail.
                let _ = thread.event_loop_sender.send(EventLoopThreadCommand::Quit);
                event_loop_thread.join();
            }),
        }
    }

    pub fn new_rpc(&self, name: &str, remote_procedure: Box<RemoteProcedure + 'a>) {
        // NOCOM(#sirver): what happens when this is already inserted? crash probably
        let mut rpc = self.call("core.new_rpc", &NewRpcRequest {
            priority: remote_procedure.priority(),
            name: name.into(),
        });
        let success = rpc.wait().unwrap();
        // NOCOM(#sirver): report failure.

        self.function_thread_sender.send(FunctionThreadCommand::NewRpc(
                name.into(), remote_procedure)).expect("NewRpc");
    }

    pub fn call<T: Serialize>(&self, function: &str, args: &T) -> RpcClientContext {
        RpcClientContext::new(&self.event_loop_sender, function, args).unwrap()
    }

    pub fn new_sender(&self) -> Sender {
        Sender {
            event_loop_sender: self.event_loop_sender.clone(),
        }
    }
}

impl<'a> Drop for Client<'a> {
    fn drop(&mut self) {
        // Either thread might have panicked at this point, so we can not rely on the sends to go
        // through. We just tell both (again) to Quit and hope they actually join.
        let _ = self.function_thread_sender.send(FunctionThreadCommand::Quit);
        let _ = self.event_loop_sender.send(EventLoopThreadCommand::Quit);
    }
}

#[derive(Clone)]
pub struct Sender {
    event_loop_sender: mio::Sender<EventLoopThreadCommand>,
}

// NOCOM(#sirver): figure out the difference between a Sender, an RpcServerContext and come up with better
// names.
impl Sender {
    pub fn call<T: Serialize>(&self, function: &str, args: &T) -> RpcClientContext {
        RpcClientContext::new(&self.event_loop_sender, function, args).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum RpcServerContextState {
    Alive,
    Finished,
    Cancelled,
}

pub struct RpcServerContext {
    context: String,
    commands: mpsc::Receiver<RpcServerContextCommand>,
    event_loop_sender: mio::Sender<EventLoopThreadCommand>,
    state: RpcServerContextState,
}

#[derive(Debug)]
pub enum RpcServerContextError {
    Finished,
    Cancelled,
    Disconnected,
}

// NOCOM(#sirver): impl error::Error for RpcServerContextError?

impl From<mio::NotifyError<EventLoopThreadCommand>> for RpcServerContextError {
    fn from(_: mio::NotifyError<EventLoopThreadCommand>) -> Self {
        RpcServerContextError::Disconnected
    }
}


pub type RpcServerContextResult<T> = result::Result<T, RpcServerContextError>;

impl RpcServerContext {
    fn new(context: String, commands: mpsc::Receiver<RpcServerContextCommand>,
           event_loop_sender: mio::Sender<EventLoopThreadCommand>) -> Self {
        RpcServerContext {
            context: context,
            commands: commands,
            event_loop_sender: event_loop_sender,
            state: RpcServerContextState::Alive
        }
    }

    fn update_state(&mut self) {
        match self.commands.try_recv() {
            Ok(value) => match value {
                RpcServerContextCommand::Cancel => self.state = RpcServerContextState::Cancelled,
            },
            Err(err) => match err {
                mpsc::TryRecvError::Empty => (),
                mpsc::TryRecvError::Disconnected => {
                    // The FunctionThread terminated - that means that the client must be shutting
                    // down. That is like we are canceled.
                    self.state = RpcServerContextState::Cancelled;
                }
            }
        }
    }
    fn check_liveness(&mut self) -> RpcServerContextResult<()> {
        self.update_state();

        match self.state {
            RpcServerContextState::Alive => Ok(()),
            RpcServerContextState::Finished => Err(RpcServerContextError::Finished),
            RpcServerContextState::Cancelled => Err(RpcServerContextError::Cancelled),
        }
    }

    pub fn call<T: Serialize>(&mut self, function: &str, args: &T) -> RpcServerContextResult<RpcClientContext> {
        try!(self.check_liveness());
        Ok(try!(RpcClientContext::new(&self.event_loop_sender, function, args)))
    }

    pub fn update<T: Serialize>(&mut self, args: &T) -> RpcServerContextResult<()> {
        try!(self.check_liveness());

        let msg = ipc::Message::RpcResponse(rpc::Response {
            context: self.context.clone(),
            kind: rpc::ResponseKind::Partial(json::to_value(args)),
        });
        Ok(try!(self.event_loop_sender.send(EventLoopThreadCommand::Send(msg))))
    }

    pub fn cancelled(&mut self) -> bool {
        self.update_state();
        self.state == RpcServerContextState::Cancelled
    }

    // NOCOM(#sirver): can consume self?
    pub fn finish(&mut self, result: rpc::Result) -> RpcServerContextResult<()> {
        try!(self.check_liveness());

        self.state = RpcServerContextState::Finished;
        let msg = ipc::Message::RpcResponse(rpc::Response {
            context: self.context.clone(),
            kind: rpc::ResponseKind::Last(result),
        });
        Ok(try!(self.event_loop_sender.send(EventLoopThreadCommand::Send(msg))))
    }
}

impl Drop for RpcServerContext {
    fn drop(&mut self) {
        match self.state {
            RpcServerContextState::Finished | RpcServerContextState::Cancelled => (),
            RpcServerContextState::Alive => panic!("RpcServerContext dropped while still alive. Call finish()!."),
        }
    }
}
