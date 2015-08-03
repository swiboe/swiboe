#![feature(test)]

extern crate serde;
extern crate switchboard;
extern crate tempdir;
extern crate test;
extern crate uuid;

#[path="../tests/support/mod.rs"] mod support;

use serde::json;
use support::TestHarness;
use switchboard::client::Client;
use switchboard::plugin_buffer;
use test::Bencher;


// On my macbook: 415,791 ns/iter (+/- 32,292)
#[bench]
fn bench_broadcast(b: &mut Bencher) {
    let t = TestHarness::new();

    let clients: Vec<_> = (1..50)
        .map(|_| Client::connect(&t.socket_name)).collect();

    let test_msg = json::builder::ObjectBuilder::new()
        .insert("blub".into(), "blah")
        .unwrap();

    b.iter(|| {
        let function_call = clients[0].call("core.broadcast", &test_msg);
        function_call.wait().unwrap();
        for client in &clients {
            assert_eq!(test_msg, client.recv().unwrap());
        }
    });
}

// On my macbook: 412,692 ns/iter (+/- 33,380)
#[bench]
fn bench_create_and_delete_buffers(b: &mut Bencher) {
    let t = TestHarness::new();
    let client = Client::connect(&t.socket_name);

    b.iter(|| {
        client.call("buffer.new", &plugin_buffer::NewRequest).wait().unwrap();

        let buffer_created: plugin_buffer::BufferCreated =
            json::from_value(client.recv().unwrap()).unwrap();
        client.call("buffer.delete", &plugin_buffer::DeleteRequest {
            buffer_index: buffer_created.buffer_index
        }).wait().unwrap();

        assert_eq!(plugin_buffer::BufferDeleted {
            buffer_index: buffer_created.buffer_index,
        }, json::from_value(client.recv().unwrap()).unwrap());
    });
}
