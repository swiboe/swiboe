#![cfg(not(test))]

extern crate serde;
extern crate switchboard;
extern crate tempdir;

#[path="../../tests/support/mod.rs"] mod support;

use serde::json;
use support::{TestHarness};
use switchboard::client::Client;
use switchboard::rpc;
use switchboard::plugin_buffer;

fn main() {
    let t = TestHarness::new();
    let active_client = Client::connect(&t.socket_name);

    loop {
        let new_response: plugin_buffer::NewResponse = match active_client.call(
            "buffer.new", &plugin_buffer::NewRequest {
                content: Some("bli\nbla\nblub".into()),
            }).wait().unwrap()
        {
            rpc::Result::Ok(value) => json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };

        let _: plugin_buffer::DeleteResponse = match active_client.call(
            "buffer.delete", &plugin_buffer::DeleteRequest {
                buffer_index: new_response.buffer_index
            }).wait().unwrap() {
            rpc::Result::Ok(value) => json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };
    }
}
