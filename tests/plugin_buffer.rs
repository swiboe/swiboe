use std::sync::atomic::{AtomicBool, Ordering};
use super::CallbackProcedure;
use support::TestHarness;
use switchboard::client;
use switchboard::ipc::RpcResult;
use switchboard::plugin_buffer;


fn create_buffer(client: &client::Client, expected_index: usize) {
    let request = plugin_buffer::NewRequest;
    let rpc = client.call("buffer.new", &request);
    assert_eq!(rpc.wait().unwrap(), RpcResult::success(plugin_buffer::NewResponse {
        buffer_index: expected_index,
    }));
}

#[test]
fn buffer_new() {
    let t = TestHarness::new();
    let callback_called  = AtomicBool::new(true);
    {
        let client = client::Client::connect(&t.socket_name);
        client.new_rpc("on.buffer.new", Box::new(CallbackProcedure {
            callback: |_| {
                callback_called.store(true, Ordering::Relaxed);
                RpcResult::success(())
            }
        }));
        create_buffer(&client, 0);
    }
    assert!(callback_called.load(Ordering::Relaxed));
}

#[test]
fn buffer_delete() {
    let t = TestHarness::new();
    let callback_called  = AtomicBool::new(true);

    {
        let client = client::Client::connect(&t.socket_name);
        client.new_rpc("on.buffer.deleted", Box::new(CallbackProcedure {
            callback: |_| {
                callback_called.store(true, Ordering::Relaxed);
                RpcResult::success(())
            }
        }));

        create_buffer(&client, 0);

        let request = plugin_buffer::DeleteRequest {
            buffer_index: 0,
        };
        let rpc = client.call("buffer.delete", &request);
        assert_eq!(rpc.wait().unwrap(), RpcResult::success(()));
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
    let rpc = client.call("buffer.delete", &request);
    assert_eq!(
        rpc.wait().unwrap().unwrap_err().details.unwrap().as_string(), Some("unknown_buffer"));
}
