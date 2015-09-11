// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::client::rpc_loop::{Command, CommandSender};
use ::error::{Error, Result};
use serde;
use serde_json;
use std::sync::mpsc;
use uuid::Uuid;

pub struct Context {
    context: String,
    values: mpsc::Receiver<::rpc::Response>,
    result: Option<::rpc::Result>,
    commands: CommandSender,
}

impl Context {
    pub fn new<T: serde::Serialize>(commands: CommandSender,
                         function: &str,
                         args: &T) -> Result<Self> {
        let args = serde_json::to_value(&args);
        let context = Uuid::new_v4().to_hyphenated_string();
        let message = ::ipc::Message::RpcCall(::rpc::Call {
            function: function.into(),
            context: context.clone(),
            args: args,
        });

        let (tx, rx) = mpsc::channel();
        // NOCOM(#sirver): this tx is only for cancelling. Maybe this can be avoided.
        // NOCOM(#sirver): the next one should be done with try!
        commands.send(Command::OutgoingCall(context.clone(), tx, message)).expect("Command::OutgoingCall");
        // NOCOM(#sirver): implement drop so that we can cancel an RPC.
        Ok(Context {
            values: rx,
            commands: commands,
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
                _ => return Err(Error::Disconnected),
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

    // NOCOM(#sirver): should this not consume the context?
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
        try!(self.commands.send(Command::CancelOutgoingRpc(self.context)));
        Ok(())
    }
}
