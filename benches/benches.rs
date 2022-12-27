// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(test)]

extern crate serde;
extern crate serde_json;
extern crate swiboe;
extern crate tempdir;
extern crate test;

use swiboe::client::{Client, RpcCaller};
use swiboe::plugin;
use swiboe::rpc;
use swiboe::testing::TestHarness;
use test::Bencher;

// On my macbook: 293,350 ns/iter (+/- 28,545)
#[bench]
fn bench_create_and_delete_buffers(b: &mut Bencher) {
    let t = TestHarness::new();
    let mut active_client = Client::connect_unix(&t.socket_name).unwrap();

    b.iter(|| {
        let new_response: plugin::buffer::new::Response = match active_client
            .call(
                "buffer.new",
                &plugin::buffer::new::Request {
                    content: Some("bli\nbla\nblub".into()),
                },
            )
            .unwrap()
            .wait()
            .unwrap()
        {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };

        let _: plugin::buffer::delete::Response = match active_client
            .call(
                "buffer.delete",
                &plugin::buffer::delete::Request {
                    buffer_index: new_response.buffer_index,
                },
            )
            .unwrap()
            .wait()
            .unwrap()
        {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };
    });
}
