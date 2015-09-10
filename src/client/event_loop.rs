// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::client::rpc_loop;
use ::ipc;
use mio::tcp::TcpStream;
use mio::unix::UnixStream;
use mio;
use std::io;
use std::thread;

const CLIENT: mio::Token = mio::Token(1);

// Each reader and writer gets their own socket, so we have to clone the file descriptors for that.
// Unfortunately, mio does not provide a trait for this already, so we make one.
pub trait TryClone: io::Read + io::Write + mio::Evented + Sized + Send {
    fn try_clone(&self) -> io::Result<Self>;
}

impl TryClone for UnixStream {
    fn try_clone(&self) -> io::Result<Self> {
        UnixStream::try_clone(&self)
    }
}

impl TryClone for TcpStream {
    fn try_clone(&self) -> io::Result<Self> {
        TcpStream::try_clone(&self)
    }
}

pub enum Command {
    Quit,
    Send(ipc::Message),
}

// NOCOM(#sirver): bad name
struct Handler<T: TryClone> {
    reader: ipc::Reader<T>,
    writer: ipc::Writer<T>,
    function_thread_sender: rpc_loop::CommandSender,
}

impl<T: TryClone> mio::Handler for Handler<T> {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::Send(message) => {
                // println!("{:?}: Client -> Server {:?}", time::precise_time_ns(), message);
                if let Err(err) = self.writer.write_message(&message) {
                    println!("Shutting down, since sending failed: {:?}", err);
                    event_loop.channel().send(Command::Quit).expect("Quit");
                }
            }
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            CLIENT => {
                if events.is_hup() {
                    println!("Server closed connection. Closing down.");
                    event_loop.channel().send(Command::Quit).expect("Command::Quit");
                    return;
                }

                if events.is_readable() {
                    loop {
                        let message;
                        match self.reader.read_message() {
                            Err(err) => {
                                println!("Shutting down, since receiving failed: {:?}", err);
                                event_loop.channel().send(Command::Quit).expect("Command::Quit");
                                return;
                            }
                            Ok(None) => break,
                            Ok(Some(msg)) => message = msg,
                        };

                        let command = rpc_loop::Command::Received(message);
                        self.function_thread_sender.send(command).expect("rpc_loop::Command::Received");
                    }
                    event_loop.reregister(
                        &self.reader.socket,
                        CLIENT,
                        mio::EventSet::readable(),
                        mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
                }
            },
            client_token => panic!("Unexpected token: {:?}", client_token),
        }
    }
}

pub fn spawn<'a, T: TryClone + 'static>(stream: T, commands_tx: rpc_loop::CommandSender)
    -> (thread::JoinHandle<()>, mio::Sender<Command>)
{
    let mut event_loop = mio::EventLoop::<Handler<T>>::new().unwrap();
    let event_loop_sender = event_loop.channel();

    let mut handler = Handler {
        reader: ipc::Reader::new(stream.try_clone().unwrap()),
        writer: ipc::Writer::new(stream),
        function_thread_sender: commands_tx,
    };
    event_loop.register(
        &handler.reader.socket, CLIENT, mio::EventSet::readable(), mio::PollOpt::edge() |
        mio::PollOpt::oneshot()).unwrap();
    let event_loop_thread = thread::spawn(move || {
        event_loop.run(&mut handler).unwrap();
    });

    (event_loop_thread, event_loop_sender)
}
