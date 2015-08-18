#![feature(test)]

extern crate serde;
extern crate serde_json;
extern crate switchboard;
extern crate tempdir;
extern crate test;

#[path="../tests/support/mod.rs"] mod support;

use support::{TestHarness};
use switchboard::client::Client;
use switchboard::plugin_buffer;
use switchboard::rpc;
use test::Bencher;


// On my macbook: 293,350 ns/iter (+/- 28,545)
#[bench]
fn bench_create_and_delete_buffers(b: &mut Bencher) {
    let t = TestHarness::new();
    let active_client = Client::connect(&t.socket_name).unwrap();

    b.iter(|| {
        let new_response: plugin_buffer::NewResponse = match active_client.call(
            "buffer.new", &plugin_buffer::NewRequest {
                content: Some("bli\nbla\nblub".into()),
            }).wait().unwrap()
        {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };

        let _: plugin_buffer::DeleteResponse = match active_client.call(
            "buffer.delete", &plugin_buffer::DeleteRequest {
                buffer_index: new_response.buffer_index
            }).wait().unwrap() {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };
    });
}
