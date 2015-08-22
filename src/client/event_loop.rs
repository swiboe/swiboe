use ::ipc;
use mio::unix::UnixStream;
use mio;
use std::collections::HashMap;
use std::sync::mpsc;
use ::client::rpc_loop;

const CLIENT: mio::Token = mio::Token(1);

pub enum Command {
    Quit,
    Send(ipc::Message),
    Call(String, mpsc::Sender<::rpc::Response>, ipc::Message),
}

// NOCOM(#sirver): bad name
struct Handler<'a> {
    reader: ipc::Reader<UnixStream>,
    writer: ipc::Writer<UnixStream>,
    running_function_calls: HashMap<String, mpsc::Sender<::rpc::Response>>,
    function_thread_sender: mpsc::Sender<rpc_loop::Command<'a>>,
}

impl<'a> Handler<'a> {
    fn send(&mut self, event_loop: &mut mio::EventLoop<Self>, message: &ipc::Message) {
        // println!("{:?}: Client -> Server {:?}", time::precise_time_ns(), message);
        if let Err(err) = self.writer.write_message(&message) {
            println!("Shutting down, since sending failed: {:?}", err);
            event_loop.channel().send(Command::Quit).expect("Quit");
        }
    }
}

impl<'a> mio::Handler for Handler<'a> {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::Send(message) => self.send(event_loop, &message),
            Command::Call(context, tx, message) => {
                self.running_function_calls.insert(context, tx);
                self.send(event_loop, &message)
            },
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
                            _ => {
                                let command = rpc_loop::Command::Received(message);
                                self.function_thread_sender.send(command).expect("rpc_loop::Command::Received");
                            },
                        }
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

pub fn spawn<'a>(stream: UnixStream, commands_tx: mpsc::Sender<rpc_loop::Command<'a>>)
    -> (::thread_scoped::JoinGuard<'a, ()>, mio::Sender<Command>)
{
    let mut event_loop = mio::EventLoop::<Handler>::new().unwrap();
    let event_loop_sender = event_loop.channel();

    let mut handler = Handler {
        reader: ipc::Reader::new(stream.try_clone().unwrap()),
        writer: ipc::Writer::new(stream),
        running_function_calls: HashMap::new(),
        function_thread_sender: commands_tx,
    };
    event_loop.register_opt(
        &handler.reader.socket, CLIENT, mio::EventSet::readable(), mio::PollOpt::edge() |
        mio::PollOpt::oneshot()).unwrap();
    let event_loop_thread = unsafe {
        ::thread_scoped::scoped(move || {
            event_loop.run(&mut handler).unwrap();
        })
    };

    (event_loop_thread, event_loop_sender)
}
