// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use error::{Error, Result};
use ipc;
use mio;
use rpc;
use server::api_table;
use server::ipc_bridge;
use server::plugin_core;
use spinner;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread;

const CORE_FUNCTIONS_PREFIX: &'static str = "core.";

pub enum Command {
    Quit,
    NewRpc(ipc_bridge::ClientId, String, u16),
    RpcCall(ipc_bridge::ClientId, rpc::Call),
    RpcResponse(rpc::Response),
    RpcCancel(rpc::Cancel),
    ClientConnected(ipc_bridge::ClientId),
    ClientDisconnected(ipc_bridge::ClientId),
    SendDataFailed(ipc_bridge::ClientId, ipc::Message, Error),
}

#[derive(Debug)]
struct RunningRpc {
    caller: ipc_bridge::ClientId,
    callee: ipc_bridge::ClientId,
    rpc_call: rpc::Call,
}

pub type SenderTo = mpsc::Sender<Command>;

pub struct Receiver {
    commands: mpsc::Receiver<Command>,
}

impl Receiver {
    pub fn new(commands: mpsc::Receiver<Command>) -> Self {
        Receiver { commands: commands }
    }
}

impl spinner::Receiver<Command> for Receiver {
    fn recv(&mut self) -> Result<Command> {
        match self.commands.recv() {
            Ok(value) => Ok(value),
            Err(_) => Err(Error::Disconnected),
        }
    }
}

pub struct Handler {
    api_table: api_table::ApiTable,
    clients: HashSet<ipc_bridge::ClientId>,
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
    running_rpcs: HashMap<String, RunningRpc>,
    plugin_core: plugin_core::CorePlugin,
}

impl Handler {
    pub fn new(
        ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
        commands_sender: SenderTo,
    ) -> Self {
        Handler {
            api_table: api_table::ApiTable::new(),
            clients: HashSet::new(),
            running_rpcs: HashMap::new(),
            ipc_bridge_commands: ipc_bridge_commands,
            plugin_core: plugin_core::CorePlugin::new(commands_sender),
        }
    }

    fn on_rpc_cancel(&mut self, rpc_cancel: rpc::Cancel) -> Result<()> {
        // NOCOM(#sirver): only the original caller can cancel, really.
        // Simply drop this message for unknown RPC
        if let Some(running_rpc) = self.running_rpcs.get(&rpc_cancel.context) {
            self.ipc_bridge_commands
                .send(ipc_bridge::Command::SendData(
                    running_rpc.callee,
                    ipc::Message::RpcCancel(rpc_cancel),
                ))?;
        }
        Ok(())
    }

    fn on_rpc_response(&mut self, rpc_response: rpc::Response) -> Result<()> {
        let mut running_rpc = match self.running_rpcs.entry(rpc_response.context.clone()) {
            Entry::Occupied(running_rpc) => running_rpc,
            Entry::Vacant(_) => {
                // Unknown RPC. We simply drop this message.
                return Ok(());
            }
        };

        match rpc_response.kind {
            rpc::ResponseKind::Partial(value) => {
                let running_rpc = running_rpc.get();
                self.ipc_bridge_commands
                    .send(ipc_bridge::Command::SendData(
                        running_rpc.caller,
                        ipc::Message::RpcResponse(rpc::Response {
                            context: running_rpc.rpc_call.context.clone(),
                            kind: rpc::ResponseKind::Partial(value),
                        }),
                    ))?;
            }
            rpc::ResponseKind::Last(result) => match result {
                rpc::Result::Ok(_) | rpc::Result::Err(_) => {
                    let running_rpc = running_rpc.remove();
                    self.ipc_bridge_commands
                        .send(ipc_bridge::Command::SendData(
                            running_rpc.caller,
                            ipc::Message::RpcResponse(rpc::Response {
                                context: running_rpc.rpc_call.context,
                                kind: rpc::ResponseKind::Last(result),
                            }),
                        ))?;
                }
                rpc::Result::NotHandled => {
                    // TODO(sirver): If a new function has been registered or been deleted since we
                    // last saw this context, this might skip a handler or call one twice. We need
                    // a better way to keep track where we are in the list of handlers.
                    let running_rpc = running_rpc.get_mut();

                    // NOCOM(#sirver): quite some code duplication with RpcCall
                    match self
                        .api_table
                        .get_next(&running_rpc.rpc_call.function, &running_rpc.callee)
                    {
                        Some(info) => {
                            // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                            // able to move again.
                            self.ipc_bridge_commands
                                .send(ipc_bridge::Command::SendData(
                                    info.client_id,
                                    ipc::Message::RpcCall(running_rpc.rpc_call.clone()),
                                ))?;
                            running_rpc.callee = info.client_id;
                        }
                        None => {
                            self.ipc_bridge_commands
                                .send(ipc_bridge::Command::SendData(
                                    running_rpc.caller,
                                    ipc::Message::RpcResponse(rpc::Response {
                                        context: running_rpc.rpc_call.context.clone(),
                                        kind: rpc::ResponseKind::Last(rpc::Result::NotHandled),
                                    }),
                                ))?;
                        }
                    };
                    // NOCOM(#sirver): we ignore timeouts.
                }
            },
        };
        Ok(())
    }
}

impl spinner::Handler<Command> for Handler {
    fn handle(&mut self, command: Command) -> Result<spinner::Command> {
        match command {
            Command::Quit => Ok(spinner::Command::Quit),
            Command::NewRpc(client_id, name, priority) => {
                // NOCOM(#sirver): deny everything starting with 'core'
                // NOCOM(#sirver): make sure the client_id is known.
                // NOCOM(#sirver): make sure the client has not already registered this
                // function.
                self.api_table.register(
                    name,
                    api_table::ApiInfo {
                        client_id: client_id,
                        priority: priority,
                    },
                );
                Ok(spinner::Command::Continue)
            }
            Command::RpcCall(client_id, rpc_call) => {
                // NOCOM(#sirver): make sure this is not already in running_rpcs.
                // NOCOM(#sirver): function name might not be in there.

                // Special case 'core.'. We handle them immediately.
                if rpc_call.function.starts_with(CORE_FUNCTIONS_PREFIX) {
                    let result = self.plugin_core.call(client_id, &rpc_call);
                    self.ipc_bridge_commands
                        .send(ipc_bridge::Command::SendData(
                            client_id,
                            ipc::Message::RpcResponse(rpc::Response {
                                context: rpc_call.context.clone(),
                                kind: rpc::ResponseKind::Last(result),
                            }),
                        ))?;
                } else {
                    match self.api_table.get_first(&rpc_call.function) {
                        Some(info) => {
                            // NOCOM(#sirver): eventually, when we keep proper track of our rpc calls, this should be
                            // able to move again.
                            self.running_rpcs.insert(
                                rpc_call.context.clone(),
                                RunningRpc {
                                    caller: client_id,
                                    callee: info.client_id,
                                    rpc_call: rpc_call.clone(),
                                },
                            );
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                info.client_id,
                                ipc::Message::RpcCall(rpc_call)
                            ))?;
                            // NOCOM(#sirver): we ignore timeouts.
                        }
                        None => {
                            self.ipc_bridge_commands.send(ipc_bridge::Command::SendData(
                                client_id,
                                ipc::Message::RpcResponse(rpc::Response {
                                    context: rpc_call.context.clone(),
                                    kind: rpc::ResponseKind::Last(rpc::Result::Err(rpc::Error {
                                        kind: rpc::ErrorKind::UnknownRpc,
                                        details: None,
                                    })),
                                })
                            ))?;
                        }
                    }
                }
                Ok(spinner::Command::Continue)
            }
            Command::RpcResponse(rpc_response) => {
                self.on_rpc_response(rpc_response)?;
                Ok(spinner::Command::Continue)
            }
            Command::RpcCancel(rpc_cancel) => {
                self.on_rpc_cancel(rpc_cancel)?;
                Ok(spinner::Command::Continue)
            }
            Command::SendDataFailed(client_id, msg, err) => {
                let action = match msg {
                    ipc::Message::RpcResponse(_) | ipc::Message::RpcCancel(_) => {
                        // NOCOM(#sirver): on a streaming rpc, this should also try to cancel
                        // the RPC.
                        "dropped the RpcResponse/RpcCall."
                    }
                    ipc::Message::RpcCall(rpc_call) => {
                        self.on_rpc_response(rpc::Response {
                            context: rpc_call.context,
                            kind: rpc::ResponseKind::Last(rpc::Result::NotHandled),
                        })?;
                        "surrogate replied as NotHandled."
                    }
                };
                println!("Sending to {:?} failed: {:?}, {}", client_id, err, action);
                Ok(spinner::Command::Continue)
            }
            Command::ClientConnected(client_id) => {
                // NOCOM(#sirver): make sure client_id is not yet known.
                self.clients.insert(client_id);
                Ok(spinner::Command::Continue)
            }
            Command::ClientDisconnected(client_id) => {
                self.clients.remove(&client_id);

                // Kill all pending RPCs that have been requested by this client.
                let rpcs_to_remove: Vec<_> = self
                    .running_rpcs
                    .iter()
                    .filter_map(|(context, running_rpc)| {
                        if running_rpc.caller == client_id {
                            Some(context.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                for context in rpcs_to_remove {
                    self.running_rpcs.remove(&context);
                }

                self.api_table.deregister_by_client(&client_id);
                Ok(spinner::Command::Continue)
            }
        }
    }
}

pub fn spawn(
    ipc_bridge_commands: mio::Sender<ipc_bridge::Command>,
    tx: SenderTo,
    rx: mpsc::Receiver<Command>,
) -> thread::JoinHandle<()> {
    let recver = Receiver::new(rx);
    let handler = Handler::new(ipc_bridge_commands, tx.clone());
    spinner::spawn(recver, handler)
}
