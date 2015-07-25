use super::buffer::Buffer;
use super::ipc::{IpcRead, IpcWrite};

use mio;
use mio::unix::{UnixListener, UnixStream};

const SERVER: mio::Token = mio::Token(0);

pub struct SupremeServer {
    buffers: Vec<Buffer>,
    unix_listener: UnixListener,
    // NOCOM(#sirver): replace conn and conns
    conns: mio::util::Slab<Connection>,
}

struct Connection {
    stream: UnixStream,
    token: mio::Token,
}

// NOCOM(#sirver): ClientConnection?
impl Connection {
    pub fn new(stream: UnixStream, token: mio::Token) -> Connection {
        Connection {
            stream: stream,
            token: token,
        }
    }
}

impl SupremeServer {
}

impl mio::Handler for SupremeServer {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<SupremeServer>, token: mio::Token, events: mio::EventSet) {
        match token {
            SERVER => {
                let stream = self.unix_listener.accept().unwrap().unwrap();
                match self.conns.insert_with(|token| {
                    println!("registering {:?} with event loop", token);
                    Connection::new(stream, token)
                }) {
                    Some(token) => {
                        // If we successfully insert, then register our connection.
                        let conn = &mut self.conns[token];
                        event_loop.register_opt(
                            &conn.stream,
                            conn.token,
                            mio::EventSet::readable(),
                            mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
                    },
                    None => {
                        // If we fail to insert, `conn` will go out of scope and be dropped.
                        panic!("Failed to insert connection into slab");
                    }
                };
                println!("Message!");
            },
            client_token => {
                if events.is_hup() {
                    self.conns.remove(client_token);
                } else if events.is_readable() {
                    let mut vec = Vec::new();
                    let conn = &mut self.conns[token];
                    conn.stream.read_message(&mut vec);
                    let s = String::from_utf8(vec).unwrap();
                    println!("#sirver s: {:#?}", s);
                    event_loop.reregister(
                        &conn.stream,
                        conn.token,
                        mio::EventSet::readable(),
                        mio::PollOpt::edge() | mio::PollOpt::oneshot()).unwrap();
                }
            }
        }
    }
}

pub fn run_supreme_server() {
    // TODO(sirver): grep for unwrap and remove
    let server = UnixListener::bind("/tmp/s.socket").unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER);

    let mut s_server = SupremeServer {
        buffers: Vec::new(),
        unix_listener: server,
        conns: mio::util::Slab::new_starting_at(mio::Token(1), 1024),
    };

    event_loop.run(&mut s_server).unwrap();
}
