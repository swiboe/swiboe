use ::client::rpc_loop;
use ::ipc;
use mio::unix::UnixStream;
use mio;
use std::thread;

const CLIENT: mio::Token = mio::Token(1);

pub enum Command {
    Quit,
    Send(ipc::Message),
}

// NOCOM(#sirver): bad name
struct Handler {
    reader: ipc::Reader<UnixStream>,
    writer: ipc::Writer<UnixStream>,
    function_thread_sender: rpc_loop::CommandSender,
}

impl mio::Handler for Handler {
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

pub fn spawn<'a>(stream: UnixStream, commands_tx: rpc_loop::CommandSender)
    -> (thread::JoinHandle<()>, mio::Sender<Command>)
{
    let mut event_loop = mio::EventLoop::<Handler>::new().unwrap();
    let event_loop_sender = event_loop.channel();

    let mut handler = Handler {
        reader: ipc::Reader::new(stream.try_clone().unwrap()),
        writer: ipc::Writer::new(stream),
        function_thread_sender: commands_tx,
    };
    event_loop.register_opt(
        &handler.reader.socket, CLIENT, mio::EventSet::readable(), mio::PollOpt::edge() |
        mio::PollOpt::oneshot()).unwrap();
    let event_loop_thread = thread::spawn(move || {
        event_loop.run(&mut handler).unwrap();
    });

    (event_loop_thread, event_loop_sender)
}
