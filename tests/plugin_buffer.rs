extern crate serde;
extern crate switchboard;

use serde::json;
use switchboard::client::{RemoteProcedure, Client};
use switchboard::ipc::RpcResultKind;
use switchboard::plugin_buffer;
use switchboard::testing::TestServer;

fn create_buffer(client: &Client, expected_index: usize) {
    let request = plugin_buffer::NewRequest;
    let rpc = client.call("buffer.new", &request);
    assert_eq!(rpc.wait().unwrap(), RpcResultKind::Ok);

    // NOCOM(#sirver): this rpc should contain information about the created buffer.

    let broadcast_msg = json::from_value(client.recv().unwrap()).unwrap();
    assert_eq!(plugin_buffer::BufferCreated {
        buffer_index: expected_index,
    }, broadcast_msg);
}

#[test]
fn buffer_new() {
    let (_server, socket_name) = TestServer::new();
    let client = Client::connect(&socket_name);
    create_buffer(&client, 0);
}

#[test]
fn buffer_delete() {
    let (_server, socket_name) = TestServer::new();
    let client = Client::connect(&socket_name);
    create_buffer(&client, 0);

    let request = plugin_buffer::DeleteRequest {
        buffer_index: 0,
    };
    let rpc = client.call("buffer.delete", &request);
    assert_eq!(rpc.wait().unwrap(), RpcResultKind::Ok);

    let broadcast_msg = json::from_value(client.recv().unwrap()).unwrap();
    assert_eq!(plugin_buffer::BufferDeleted {
        buffer_index: 0,
    }, broadcast_msg);

    // NOCOM(#sirver): add a test for non existing buffer

}
