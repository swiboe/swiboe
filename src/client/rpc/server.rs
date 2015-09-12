// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::client::rpc_loop;
use ::error::{Result, Error, ErrorKind};
use serde::Serialize;
use serde_json;
use std::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
enum ContextState {
    Alive,
    Finished,
    Cancelled,
}

pub enum Command {
    Cancel,
}

pub trait Rpc: Send + Sync {
    fn priority(&self) -> u16 { u16::max_value() }
    fn call(&self, context: Context, args: serde_json::Value);
}

pub struct Context {
    context: String,
    commands: mpsc::Receiver<Command>,
    rpc_loop_commands: rpc_loop::CommandSender,
    state: ContextState,
}

impl Context {
    pub fn new(context: String, commands: mpsc::Receiver<Command>,
           rpc_loop_commands: rpc_loop::CommandSender) -> Self {
        Context {
            context: context,
            commands: commands,
            rpc_loop_commands: rpc_loop_commands,
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
            ContextState::Finished => Err(Error::new(ErrorKind::RpcAlreadyFinished)),
            ContextState::Cancelled => Err(Error::new(ErrorKind::RpcWasCancelled)),
        }
    }

    pub fn call<T: Serialize>(&mut self, function: &str, args: &T) -> Result<::client::rpc::client::Context> {
        try!(self.check_liveness());
        Ok(try!(::client::rpc::client::Context::new(self.rpc_loop_commands.clone(), function, args)))
    }

    pub fn update<T: Serialize>(&mut self, args: &T) -> Result<()> {
        try!(self.check_liveness());

        let msg = ::ipc::Message::RpcResponse(::rpc::Response {
            context: self.context.clone(),
            kind: ::rpc::ResponseKind::Partial(serde_json::to_value(args)),
        });
        Ok(try!(self.rpc_loop_commands.send(rpc_loop::Command::Send(msg))))
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
        Ok(try!(self.rpc_loop_commands.send(rpc_loop::Command::Send(msg))))
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
