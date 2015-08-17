use ::client::event_loop;
use mio;
use serde::Serialize;
use serde_json;
use std::result;
use std::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
enum ContextState {
    Alive,
    Finished,
    Cancelled,
}

#[derive(Debug)]
pub enum Error {
    Finished,
    Cancelled,
    Disconnected,
}

// NOCOM(#sirver): impl error::Error for Error?

impl From<mio::NotifyError<event_loop::Command>> for Error {
    fn from(_: mio::NotifyError<event_loop::Command>) -> Self {
        Error::Disconnected
    }
}

pub enum Command {
    Cancel,
}

pub trait Rpc: Send {
    fn priority(&self) -> u16 { u16::max_value() }
    fn call(&mut self, context: Context, args: serde_json::Value);
}

pub type Result<T> = result::Result<T, Error>;

pub struct Context {
    context: String,
    commands: mpsc::Receiver<Command>,
    event_loop_sender: mio::Sender<event_loop::Command>,
    state: ContextState,
}

impl Context {
    pub fn new(context: String, commands: mpsc::Receiver<Command>,
           event_loop_sender: mio::Sender<event_loop::Command>) -> Self {
        Context {
            context: context,
            commands: commands,
            event_loop_sender: event_loop_sender,
            state: ContextState::Alive
        }
    }

    fn update_state(&mut self) {
        match self.commands.try_recv() {
            Ok(value) => match value {
                Command::Cancel => self.state = ContextState::Cancelled,
            },
            Err(err) => match err {
                mpsc::TryRecvError::Empty => (),
                mpsc::TryRecvError::Disconnected => {
                    // The FunctionThread terminated - that means that the client must be shutting
                    // down. That is like we are canceled.
                    self.state = ContextState::Cancelled;
                }
            }
        }
    }

    fn check_liveness(&mut self) -> Result<()> {
        self.update_state();

        match self.state {
            ContextState::Alive => Ok(()),
            ContextState::Finished => Err(Error::Finished),
            ContextState::Cancelled => Err(Error::Cancelled),
        }
    }

    pub fn call<T: Serialize>(&mut self, function: &str, args: &T) -> Result<::client::rpc::client::Context> {
        try!(self.check_liveness());
        Ok(try!(::client::rpc::client::Context::new(&self.event_loop_sender, function, args)))
    }

    pub fn update<T: Serialize>(&mut self, args: &T) -> Result<()> {
        try!(self.check_liveness());

        let msg = ::ipc::Message::RpcResponse(::rpc::Response {
            context: self.context.clone(),
            kind: ::rpc::ResponseKind::Partial(serde_json::to_value(args)),
        });
        Ok(try!(self.event_loop_sender.send(event_loop::Command::Send(msg))))
    }

    pub fn cancelled(&mut self) -> bool {
        self.update_state();
        self.state == ContextState::Cancelled
    }

    // NOCOM(#sirver): can consume self?
    pub fn finish(&mut self, result: ::rpc::Result) -> Result<()> {
        try!(self.check_liveness());

        self.state = ContextState::Finished;
        let msg = ::ipc::Message::RpcResponse(::rpc::Response {
            context: self.context.clone(),
            kind: ::rpc::ResponseKind::Last(result),
        });
        Ok(try!(self.event_loop_sender.send(event_loop::Command::Send(msg))))
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        match self.state {
            ContextState::Finished | ContextState::Cancelled => (),
            ContextState::Alive => panic!("Context dropped while still alive. Call finish()!."),
        }
    }
}
