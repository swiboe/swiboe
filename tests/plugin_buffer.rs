// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use std::sync::{Arc, Mutex};
use std::thread;
use swiboe::client;
use swiboe::client::RpcCaller;
use swiboe::plugin::buffer;
use swiboe::rpc;
use swiboe::testing::TestHarness;
use {create_file, CallbackRpc};

fn wait_for_true_with_timeout(mutex: &Mutex<bool>) -> bool {
    for _ in 0..10 {
        if *mutex.lock().unwrap() {
            return true;
        }
        thread::sleep_ms(50);
    }
    false
}

fn create_buffer(client: &mut client::Client, expected_index: usize, content: Option<&str>) {
    let request = buffer::new::Request {
        content: content.map(|s| s.to_string()),
    };
    let mut rpc = client.call("buffer.new", &request).unwrap();
    assert_eq!(
        rpc.wait().unwrap(),
        rpc::Result::success(buffer::new::Response {
            buffer_index: expected_index,
        })
    );
}

#[test]
fn buffer_new() {
    let t = TestHarness::new();
    let callback_called = Arc::new(Mutex::new(false));
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();
    {
        let callback_called = callback_called.clone();
        client
            .new_rpc(
                "on.buffer.new",
                Box::new(CallbackRpc {
                    priority: 100,
                    callback: move |mut sender: client::rpc::server::Context, _| {
                        sender.finish(rpc::Result::success(())).unwrap();
                        *callback_called.lock().unwrap() = true;
                    },
                }),
            )
            .unwrap();

        create_buffer(&mut client, 0, None);
    }
    assert!(wait_for_true_with_timeout(&*callback_called));
}

#[test]
fn buffer_new_with_content() {
    let t = TestHarness::new();
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();

    let content = "blub\nblah\nbli";
    create_buffer(&mut client, 0, Some(content));

    let mut rpc = client
        .call(
            "buffer.get_content",
            &buffer::get_content::Request { buffer_index: 0 },
        )
        .unwrap();
    assert_eq!(
        rpc.wait().unwrap(),
        rpc::Result::success(buffer::get_content::Response {
            content: content.into(),
        })
    );
}

#[test]
fn buffer_open_unhandled_uri() {
    let t = TestHarness::new();
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();

    let mut rpc = client
        .call(
            "buffer.open",
            &buffer::open::Request {
                uri: "blumba://foo".into(),
            },
        )
        .unwrap();

    assert_eq!(rpc::Result::NotHandled, rpc.wait().unwrap());
}

#[test]
fn buffer_open_file() {
    let t = TestHarness::new();
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();

    let content = "blub\nblah\nbli";
    let path = create_file(&t, "foo", &content);

    let mut rpc = client
        .call(
            "buffer.open",
            &buffer::open::Request {
                uri: format!("file://{}", path.to_str().unwrap()),
            },
        )
        .unwrap();
    assert_eq!(
        rpc.wait().unwrap(),
        rpc::Result::success(buffer::open::Response { buffer_index: 0 })
    );

    let mut rpc = client
        .call(
            "buffer.get_content",
            &buffer::get_content::Request { buffer_index: 0 },
        )
        .unwrap();
    assert_eq!(
        rpc.wait().unwrap(),
        rpc::Result::success(buffer::get_content::Response {
            content: content.into(),
        })
    );
}

#[test]
fn buffer_delete() {
    let t = TestHarness::new();
    let callback_called = Arc::new(Mutex::new(false));
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();
    {
        let callback_called = callback_called.clone();
        client
            .new_rpc(
                "on.buffer.deleted",
                Box::new(CallbackRpc {
                    priority: 100,
                    callback: move |mut sender: client::rpc::server::Context, _| {
                        sender.finish(rpc::Result::success(())).unwrap();
                        *callback_called.lock().unwrap() = true;
                    },
                }),
            )
            .unwrap();

        create_buffer(&mut client, 0, None);

        let request = buffer::delete::Request { buffer_index: 0 };
        let mut rpc = client.call("buffer.delete", &request).unwrap();
        assert_eq!(rpc.wait().unwrap(), rpc::Result::success(()));
    }
    assert!(wait_for_true_with_timeout(&*callback_called));
}

#[test]
fn buffer_delete_non_existing() {
    let t = TestHarness::new();

    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();
    let request = buffer::delete::Request { buffer_index: 0 };
    let mut rpc = client.call("buffer.delete", &request).unwrap();
    assert_eq!(
        rpc.wait()
            .unwrap()
            .unwrap_err()
            .details
            .unwrap()
            .as_str(),
        Some("unknown_buffer")
    );
}
