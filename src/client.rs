use mio::unix::UnixStream;
use mio;
use std::io::Write;
use super::ipc::{IpcRead, IpcWrite};
use std::thread;

const CLIENT: mio::Token = mio::Token(1);

pub struct SupremeClient {
    unix_stream: UnixStream,
}

impl SupremeClient {
}

impl mio::Handler for SupremeClient {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut mio::EventLoop<SupremeClient>, token: mio::Token, _: mio::EventSet) {
        match token {
            CLIENT => {
                println!("Message!");
            },
            _ => {
                panic!("Unexpected token {:?}", token);
            }
        }
    }
}

pub fn run_supreme_client() {
    let mut client = UnixStream::connect("/tmp/s.socket").unwrap();

    for i in (0..10) {
        client.write_message("hello world.".as_bytes());
        thread::sleep_ms(500);
    }
    // let mut event_loop = mio::EventLoop::new().unwrap();
    // event_loop.register(&client, CLIENT);

    // let mut s_client = SupremeClient {
        // unix_stream: client,
    // };

    // event_loop.run(&mut s_client).unwrap();
}
