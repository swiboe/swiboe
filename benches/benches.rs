#![feature(test)]

extern crate serde;
extern crate switchboard;
extern crate tempdir;
extern crate test;

#[path="../tests/support/mod.rs"] mod support;

use serde::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use support::{CallbackProcedure, TestHarness};
use switchboard::client::Client;
use switchboard::ipc;
use switchboard::plugin_buffer;
use test::Bencher;


// On my macbook: 412,692 ns/iter (+/- 33,380)
#[bench]
fn bench_create_and_delete_buffers(b: &mut Bencher) {
    let num_created = AtomicUsize::new(0);
    let num_deleted = AtomicUsize::new(0);

    {
        let t = TestHarness::new();
        let active_client = Client::connect(&t.socket_name);
        let callback_client = Client::connect(&t.socket_name);

        let num_created_move = &num_created;
        callback_client.new_rpc("on.buffer.new", Box::new(CallbackProcedure {
            callback: move |_| {
                num_created_move.fetch_add(1, Ordering::Relaxed);
                ipc::RpcResult::success(())
            }
        }));

        let num_deleted_move = &num_deleted;
        callback_client.new_rpc("on.buffer.deleted", Box::new(CallbackProcedure {
            callback: move |_| {
                num_deleted_move.fetch_add(1, Ordering::Relaxed);
                ipc::RpcResult::success(())
            }
        }));

        b.iter(|| {
            let new_response: plugin_buffer::NewResponse = match active_client.call(
                "buffer.new", &plugin_buffer::NewRequest).wait().unwrap()
            {
                ipc::RpcResult::Ok(value) => json::from_value(value).unwrap(),
                err => panic!("{:?}", err),
            };

            let _: plugin_buffer::DeleteResponse = match active_client.call(
                "buffer.delete", &plugin_buffer::DeleteRequest {
                    buffer_index: new_response.buffer_index
                }).wait().unwrap() {
                ipc::RpcResult::Ok(value) => json::from_value(value).unwrap(),
                err => panic!("{:?}", err),
            };
        });

    }
    let num_creates = num_created.load(Ordering::Relaxed);
    let num_deletes = num_created.load(Ordering::Relaxed);
    assert!(num_creates > 0);
    assert_eq!(num_creates, num_deletes);
}
