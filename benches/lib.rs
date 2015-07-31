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

// NOCOM(#sirver): duplicated - pull out stuff into a library
fn temporary_socket_name() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("{}.socket", Uuid::new_v4().to_string()));
    dir
}

#[bench]
fn bench_broadcast(b: &mut Bencher) {
    let socket_name = temporary_socket_name();
    let mut server = Server::launch(&socket_name);

    // NOCOM(#sirver): making the num clients to high yields a crash?
    let clients: Vec<_> = (1..5)
        .map(|_| Client::connect(&socket_name.to_string_lossy())).collect();

    let test_msg = json::builder::ObjectBuilder::new()
        .insert("blub".into(), "blah")
        .unwrap();

    b.iter(|| {
        let function_call = clients[0].call("core.broadcast", &test_msg);
        function_call.wait().unwrap();
        let msg = clients[0].recv().unwrap();
        for client in &clients[1..] {
            let msg1 = client.recv().unwrap();
            assert_eq!(msg, msg1);
        }
    });

    server.shutdown();
}
