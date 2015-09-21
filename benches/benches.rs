// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(test)]

extern crate serde;
extern crate serde_json;
extern crate swiboe;
extern crate tempdir;
extern crate test;

use swiboe::client::{RpcCaller, Client};
use swiboe::plugin_buffer;
use swiboe::rpc;
use swiboe::testing::TestHarness;
use test::Bencher;


// On my macbook: 293,350 ns/iter (+/- 28,545)
#[bench]
fn bench_create_and_delete_buffers(b: &mut Bencher) {
    let t = TestHarness::new();
    let mut active_client = Client::connect_unix(&t.socket_name).unwrap();

    b.iter(|| {
        let new_response: plugin_buffer::NewResponse = match active_client.call(
            "buffer.new", &plugin_buffer::NewRequest {
                content: Some("bli\nbla\nblub".into()),
            }).unwrap().wait().unwrap()
        {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };

        let _: plugin_buffer::DeleteResponse = match active_client.call(
            "buffer.delete", &plugin_buffer::DeleteRequest {
                buffer_index: new_response.buffer_index
            }).unwrap().wait().unwrap() {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };
    });
}
