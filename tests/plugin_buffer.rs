use ::{CallbackRpc, create_file};
use std::sync::atomic::{AtomicBool, Ordering};
use support::TestHarness;
use switchboard::client;
use switchboard::plugin_buffer;
use switchboard::rpc;


fn create_buffer(client: &client::Client, expected_index: usize, content: Option<&str>) {
    let request = plugin_buffer::NewRequest {
        content: content.map(|s| s.to_string()),
    };
    let mut rpc = client.call("buffer.new", &request);
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::NewResponse {
        buffer_index: expected_index,
    }));
}

#[test]
fn buffer_new() {
    let t = TestHarness::new();
    let callback_called  = AtomicBool::new(true);
    {
        let client = client::Client::connect(&t.socket_name);
        client.new_rpc("on.buffer.new", Box::new(CallbackRpc {
            priority: 100,
            callback: |mut sender: client::rpc::server::Context, _| {
                callback_called.store(true, Ordering::Relaxed);
                sender.finish(rpc::Result::success(())).unwrap();
            }
        }));
        create_buffer(&client, 0, None);
    }
    assert!(callback_called.load(Ordering::Relaxed));
}

#[test]
fn buffer_new_with_content() {
    let t = TestHarness::new();
    let client = client::Client::connect(&t.socket_name);

    let content = "blub\nblah\nbli";
    create_buffer(&client, 0, Some(content));

    let mut rpc = client.call("buffer.get_content", &plugin_buffer::GetContentRequest {
        buffer_index: 0,
    });
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::GetContentResponse {
        content: content.into(),
    }));
}

#[test]
fn buffer_open_unhandled_uri() {
    let t = TestHarness::new();
    let client = client::Client::connect(&t.socket_name);

    let mut rpc = client.call("buffer.open", &plugin_buffer::OpenRequest {
        uri: "blumba://foo".into(),
    });

    assert_eq!(rpc::Result::NotHandled, rpc.wait().unwrap());
}

#[test]
fn buffer_open_file() {
    let t = TestHarness::new();
    let client = client::Client::connect(&t.socket_name);

    let content = "blub\nblah\nbli";
    let path = create_file(&t, "foo", &content);

    let mut rpc = client.call("buffer.open", &plugin_buffer::OpenRequest {
        uri: format!("file://{}", path.to_str().unwrap()),
    });
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::OpenResponse {
        buffer_index: 0,
    }));

    let mut rpc = client.call("buffer.get_content", &plugin_buffer::GetContentRequest {
        buffer_index: 0,
    });
    assert_eq!(rpc.wait().unwrap(), rpc::Result::success(plugin_buffer::GetContentResponse {
        content: content.into(),
    }));
}

#[test]
fn buffer_delete() {
    let t = TestHarness::new();
    let callback_called  = AtomicBool::new(false);

    {
        let client = client::Client::connect(&t.socket_name);
        client.new_rpc("on.buffer.deleted", Box::new(CallbackRpc {
            priority: 100,
            callback: |mut sender: client::rpc::server::Context, _| {
                callback_called.store(true, Ordering::Relaxed);
                sender.finish(rpc::Result::success(())).unwrap();
            }
        }));

        create_buffer(&client, 0, None);

        let request = plugin_buffer::DeleteRequest {
            buffer_index: 0,
        };
        let mut rpc = client.call("buffer.delete", &request);
        assert_eq!(rpc.wait().unwrap(), rpc::Result::success(()));
    }
    assert!(callback_called.load(Ordering::Relaxed));
}

#[test]
fn buffer_delete_non_existing() {
    let t = TestHarness::new();

    let client = client::Client::connect(&t.socket_name);
    let request = plugin_buffer::DeleteRequest {
        buffer_index: 0,
    };
    let mut rpc = client.call("buffer.delete", &request);
    assert_eq!(
        rpc.wait().unwrap().unwrap_err().details.unwrap().as_string(), Some("unknown_buffer"));
}
