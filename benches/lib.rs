#![feature(test)]

extern crate serde;
extern crate switchboard;
extern crate test;
extern crate uuid;

use serde::json;
use std::env;
use std::path::{PathBuf};
use switchboard::client::Client;
use switchboard::server::{Server};
use test::Bencher;
use uuid::Uuid;

// NOCOM(#sirver): use the name switchboard everywhere.

// NOCOM(#sirver): duplicated
fn temporary_socket_name() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("{}.socket", Uuid::new_v4().to_string()));
    dir
}

#[bench]
fn bench_broadcast(b: &mut Bencher) {
    let socket_name = temporary_socket_name();
    let mut s = Server::launch(&socket_name);

    let mut client1 = Client::connect(&socket_name.to_string_lossy());
    let mut client2 = Client::connect(&socket_name.to_string_lossy());

    let test_msg = json::builder::ObjectBuilder::new()
        .insert("blub".into(), "blah")
        .unwrap();

    b.iter(|| {
        let function_call = client1.call("core.broadcast", &test_msg);
        function_call.wait();
        let b1 = client1.recv();
        let b2 = client2.recv();
        assert_eq!(b1, b2);
    });

    s.shutdown();
}
