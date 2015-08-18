use ::client::event_loop::Command;
use mio;
use serde;
use serde_json;
use std::sync::mpsc;
use uuid::Uuid;

pub struct Context {
    context: String,
    values: mpsc::Receiver<::rpc::Response>,
    result: Option<::rpc::Result>,
    event_loop_sender: mio::Sender<Command>,
}

#[derive(Debug)]
pub enum Error {
    Disconnected,
    InvalidOrUnexpectedReply(serde_json::Error),
}

// NOCOM(#sirver): impl error::Error for Error?

impl From<mio::NotifyError<Command>> for Error {
    fn from(_: mio::NotifyError<Command>) -> Self {
        Error::Disconnected
    }
}

impl From<mpsc::RecvError> for Error {
    fn from(_: mpsc::RecvError) -> Self {
        Error::Disconnected
    }
}

impl From<serde_json::error::Error> for Error {
     fn from(error: serde_json::error::Error) -> Self {
         Error::InvalidOrUnexpectedReply(error)
     }
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl Context {
    pub fn new<T: serde::Serialize>(event_loop_sender: &mio::Sender<Command>,
                         function: &str,
                         args: &T) -> ::std::result::Result<Self, mio::NotifyError<Command>> {
        let args = serde_json::to_value(&args);
        let context = Uuid::new_v4().to_hyphenated_string();
        let message = ::ipc::Message::RpcCall(::rpc::Call {
            function: function.into(),
            context: context.clone(),
            args: args,
        });

        let (tx, rx) = mpsc::channel();
        try!(event_loop_sender.send(Command::Call(context.clone(), tx, message)));
        // NOCOM(#sirver): implement drop so that we can cancel an RPC.
        Ok(Context {
            values: rx,
            event_loop_sender: event_loop_sender.clone(),
            context: context,
            result: None,
        })
    }

    pub fn try_recv(&mut self) -> Result<Option<serde_json::Value>> {
        if self.result.is_some() {
            return Ok(None);
        }

        let rpc_response = match self.values.try_recv() {
            Ok(value) => value,
            Err(err) => match err {
                mpsc::TryRecvError::Empty => return Ok(None),
                err => return Err(Error::Disconnected),
            }
        };

        match rpc_response.kind {
            ::rpc::ResponseKind::Partial(value) => Ok(Some(value)),
            ::rpc::ResponseKind::Last(result) => {
                self.result = Some(result);
                Ok(None)
            },
        }
    }

    // NOCOM(#sirver): timeout?
    pub fn recv(&mut self) -> Result<Option<serde_json::Value>> {
        if self.result.is_some() {
            return Ok(None);
        }

        let rpc_response = try!(self.values.recv());
        match rpc_response.kind {
            ::rpc::ResponseKind::Partial(value) => Ok(Some(value)),
            ::rpc::ResponseKind::Last(result) => {
                self.result = Some(result);
                Ok(None)
            },
        }
    }

    pub fn wait(&mut self) -> Result<::rpc::Result> {
        while let Some(_) = try!(self.recv()) {
        }
        Ok(self.result.take().unwrap())
    }

    pub fn done(&self) -> bool {
        self.result.is_some()
    }

    pub fn wait_for<T: serde::Deserialize>(&mut self) -> Result<T> {
        match try!(self.wait()) {
            ::rpc::Result::Ok(value) => Ok(try!(serde_json::from_value(value))),
            ::rpc::Result::Err(err) => panic!("#sirver err: {:#?}", err),
            // NOCOM(#sirver): probably should ignore other errors.
            other => panic!("#sirver other: {:#?}", other),
        }
    }

    pub fn cancel(self) -> Result<()> {
        let msg = ::ipc::Message::RpcCancel(::rpc::Cancel {
            context: self.context.clone(),
        });
        try!(self.event_loop_sender.send(Command::Send(msg)));
        Ok(())
    }
}
