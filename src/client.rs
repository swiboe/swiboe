use serde::json;

// NOCOM(#sirver): use a custom enum for error codes even in json.

use mio::unix::UnixStream;
use mio;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use super::ipc::{IpcWrite, IpcRead};
use super::server::{RpcReply, RpcState, RpcResultKind};
use uuid::Uuid;

const CLIENT: mio::Token = mio::Token(1);

pub struct Rpc {
    // NOCOM(#sirver): something more structured?
    pub values: mpsc::Receiver<json::Value>,
}

impl Rpc {
    fn new(values: mpsc::Receiver<json::Value>) -> Self {
        // NOCOM(#sirver): implement drop so that we can cancel an RPC.
        Rpc {
            values: values,
        }
    }

    // NOCOM(#sirver): timeout?
    fn recv(&self) -> RpcReply {
        let value = self.values.recv().map_err(|err| {
            panic!("Unexpected error in recv: {}", err);
        }).unwrap();

        // NOCOM(#sirver): could be faulty
        json::from_value(value).unwrap()
    }

    pub fn wait(self) -> RpcResultKind {
        loop {
            let rpc_reply = self.recv();
            if rpc_reply.state == RpcState::Done {
                return rpc_reply.result;
            }
            // NOCOM(#sirver): put data into queue.
        }
        RpcResultKind::Ok
    }
}

pub struct Client {
    values: mpsc::Receiver<json::Value>,
    event_loop_thread: thread::JoinHandle<()>,
    // NOCOM(#sirver): ipc_brigde_comands with only one m
    network_commands: mio::Sender<Command>,
}

pub enum Command {
    Quit,
    SendData(String),
    Call(String, mpsc::Sender<json::Value>),
}

// NOCOM(#sirver): bad name
struct Handler {
    stream: UnixStream,
    values: mpsc::Sender<json::Value>,
    running_function_calls: HashMap<String, mpsc::Sender<json::Value>>,
}

impl mio::Handler for Handler {
    type Timeout = ();
    type Message = Command;

    fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, command: Command) {
        match command {
            Command::Quit => event_loop.shutdown(),
            Command::SendData(data) => {
                self.stream.write_message(data.as_bytes());
            },
            Command::Call(context, tx) => {
                self.running_function_calls.insert(context, tx);
            }
        }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet) {
        match token {
            CLIENT => {
                // NOCOM(#sirver): what about hup?
                if events.is_readable() {
                    let mut vec = Vec::new();
                    self.stream.read_message(&mut vec);
                    let s = String::from_utf8(vec).unwrap();
                    println!("#sirver s: {:#?}", s);
                    let value: json::Value = json::from_str(&s).unwrap();

                    let channel = match value.find("context") {
                        Some(context) => {
                            let context = &context.as_string().unwrap() as &str;
                            self.running_function_calls.get(context)
                        }
                        None => Some(&self.values),
                    };
                    channel.map(|channel| {
                        channel.send(value).unwrap();
                    });
                }
            },
            client_token => panic!("Unexpected token: {:?}", client_token),
        }
    }
}

impl Client {
    // NOCOM(#sirver): socket_name should be a path
    pub fn connect(socket_name: &str) -> Self {
        let mut stream =
            UnixStream::connect(socket_name).unwrap();

        // NOCOM(#sirver): what
        // client.write_message("{ \"type\": \"call\", \"function\": \"core.exit\" }".as_bytes());
        // thread::sleep_ms(1500);
        let mut event_loop = mio::EventLoop::new().unwrap();
        event_loop.register_opt(
                            &stream,
                            CLIENT,
                            mio::EventSet::readable(),
                            mio::PollOpt::level()).unwrap();

        let (client_tx, values) = mpsc::channel();
        let network_commands = event_loop.channel();
        let event_loop_thread = thread::spawn(move || {
            event_loop.run(&mut Handler {
                stream: stream,
                values: client_tx,
                running_function_calls: HashMap::new(),
            }).unwrap();
        });

        let mut client = Client {
            values: values,
            network_commands: network_commands,
            event_loop_thread: event_loop_thread,
        };
        client
    }

    pub fn write(&self, data: &json::Value) {
        self.network_commands.send(
            Command::SendData(json::to_string(&data).unwrap())).unwrap();
    }

    pub fn recv(&mut self) -> json::Value {
        match self.values.recv() {
            Ok(value) => value,
            Err(err) => panic!("Disconnected."),
        }
    }

    // NOCOM(#sirver): Return a future? How about streaming functions?
    pub fn call(&mut self, function: &str, args: &json::Value) -> Rpc {
        let context = Uuid::new_v4().to_hyphenated_string();

        // NOCOM(#sirver): make this a struct.
        let data = json::builder::ObjectBuilder::new()
            .insert("function".into(), function)
            .insert("type".into(), "call")
            .insert("context".into(), &context)
            .insert("args".into(), args)
            .unwrap();

        let (tx, rx) = mpsc::channel();
        self.network_commands.send(Command::Call(context, tx)).unwrap();

        self.write(&data);
        Rpc::new(rx)
    }
}
