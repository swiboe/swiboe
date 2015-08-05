use std::sync::mpsc;
use support::{TestHarness, CallbackProcedure};
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
    let client = client::Client::connect(&t.socket_name);

    // TODO(sirver): this test will never fail, it will just wait forever on the channel.
    let (tx, rx) = mpsc::channel();
    client.new_rpc("on.buffer.new", Box::new(CallbackProcedure {
        callback: move |_| {
            tx.send(true).unwrap();
            RpcResult::success(())
        }
    }));

    create_buffer(&client, 0);
    rx.recv().unwrap();
}

#[test]
fn buffer_delete() {
    let t = TestHarness::new();
    let client = client::Client::connect(&t.socket_name);

    // TODO(sirver): this test will never fail, it will just wait forever on the channel.
    let (tx, rx) = mpsc::channel();
    client.new_rpc("on.buffer.deleted", Box::new(CallbackProcedure {
        callback: move |_| {
            tx.send(true).unwrap();
            RpcResult::success(())
        }
    }));

    create_buffer(&client, 0);

    let request = plugin_buffer::DeleteRequest {
        buffer_index: 0,
    };
    let rpc = client.call("buffer.delete", &request);
    assert_eq!(rpc.wait().unwrap(), RpcResult::success(()));

    rx.recv().unwrap();

    // NOCOM(#sirver): add a test for non existing buffer

}
