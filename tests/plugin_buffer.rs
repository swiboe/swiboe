// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

use ::{CallbackRpc, create_file};
use std::sync::{Arc, Mutex};
use support::TestHarness;
use swiboe::client::RpcCaller;
use swiboe::client;
use swiboe::plugin_buffer;
use swiboe::rpc;

fn create_buffer(client: &mut client::Client, expected_index: usize, content: Option<&str>) {
    let request = plugin_buffer::NewRequest {
        content: content.map(|s| s.to_string()),
    };
    let mut rpc = client.call("buffer.new", &request).unwrap();
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::NewResponse {
        buffer_index: expected_index,
    }));
}

#[test]
fn buffer_new() {
    let t = TestHarness::new();
    let callback_called = Arc::new(Mutex::new(false));
    {
        let mut client = client::Client::connect_unix(&t.socket_name).unwrap();
        let callback_called = callback_called.clone();
        client.new_rpc("on.buffer.new", Box::new(CallbackRpc {
            priority: 100,
            callback: move |mut sender: client::rpc::server::Context, _| {
                *callback_called.lock().unwrap() = true;
                sender.finish(rpc::Result::success(())).unwrap();
            }
        })).unwrap();
        create_buffer(&mut client, 0, None);
    }
    assert!(*callback_called.lock().unwrap());
}

#[test]
fn buffer_new_with_content() {
    let t = TestHarness::new();
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();

    let content = "blub\nblah\nbli";
    create_buffer(&mut client, 0, Some(content));

    let mut rpc = client.call("buffer.get_content", &plugin_buffer::GetContentRequest {
        buffer_index: 0,
    }).unwrap();
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::GetContentResponse {
        content: content.into(),
    }));
}

#[test]
fn buffer_open_unhandled_uri() {
    let t = TestHarness::new();
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();

    let mut rpc = client.call("buffer.open", &plugin_buffer::OpenRequest {
        uri: "blumba://foo".into(),
    }).unwrap();

    assert_eq!(rpc::Result::NotHandled, rpc.wait().unwrap());
}

#[test]
fn buffer_open_file() {
    let t = TestHarness::new();
    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();

    let content = "blub\nblah\nbli";
    let path = create_file(&t, "foo", &content);

    let mut rpc = client.call("buffer.open", &plugin_buffer::OpenRequest {
        uri: format!("file://{}", path.to_str().unwrap()),
    }).unwrap();
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::OpenResponse {
        buffer_index: 0,
    }));

    let mut rpc = client.call("buffer.get_content", &plugin_buffer::GetContentRequest {
        buffer_index: 0,
    }).unwrap();
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::GetContentResponse {
        content: content.into(),
    }));
}

#[test]
fn buffer_delete() {
    let t = TestHarness::new();
    let callback_called = Arc::new(Mutex::new(false));
    {
        let mut client = client::Client::connect_unix(&t.socket_name).unwrap();
        let callback_called = callback_called.clone();
        client.new_rpc("on.buffer.deleted", Box::new(CallbackRpc {
            priority: 100,
            callback: move |mut sender: client::rpc::server::Context, _| {
                *callback_called.lock().unwrap() = true;
                sender.finish(rpc::Result::success(())).unwrap();
            }
        })).unwrap();

        create_buffer(&mut client, 0, None);

        let request = plugin_buffer::DeleteRequest {
            buffer_index: 0,
        };
        let mut rpc = client.call("buffer.delete", &request).unwrap();
        assert_eq!(rpc.wait().unwrap(), rpc::Result::success(()));
    }
    assert!(*callback_called.lock().unwrap());
}

#[test]
fn buffer_delete_non_existing() {
    let t = TestHarness::new();

    let mut client = client::Client::connect_unix(&t.socket_name).unwrap();
    let request = plugin_buffer::DeleteRequest {
        buffer_index: 0,
    };
    let mut rpc = client.call("buffer.delete", &request).unwrap();
    assert_eq!(
        rpc.wait().unwrap().unwrap_err().details.unwrap().as_string(), Some("unknown_buffer"));
}
