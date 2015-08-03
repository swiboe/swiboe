// NOCOM(#sirver): is that needed?
#![feature(test)]

extern crate serde;
extern crate switchboard;
extern crate test;

use serde::json;
use support::TestServer;
use switchboard::client::Client;
use switchboard::plugin_buffer;
use test::Bencher;


// On my macbook: 210,033 ns/iter (+/- 20,747)
#[bench]
fn bench_broadcast(b: &mut Bencher) {
    let (_server, socket_name) = TestServer::new();

    // Increasing the number of clients makes my system run out of file descriptors really quickly.
    let clients: Vec<_> = (1..5)
        .map(|_| Client::connect(&socket_name)).collect();

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

#[bench]
fn bench_create_and_delete_buffers(b: &mut Bencher) {
    let (_server, socket_name) = TestServer::new();
    let client = Client::connect(&socket_name);

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
