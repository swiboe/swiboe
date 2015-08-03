extern crate serde;
extern crate switchboard;

use serde::json;
use switchboard::client::{RemoteProcedure, Client};
use switchboard::ipc::RpcResultKind;
use switchboard::plugin_buffer;
use switchboard::testing::TestServer;

#[test]
fn buffer_new() {
    let (_server, socket_name) = TestServer::new();
    let client = Client::connect(&socket_name);

    let request = plugin_buffer::NewRequest;

    let rpc = client.call("buffer.new", &request);
    assert_eq!(rpc.wait().unwrap(), RpcResultKind::Ok);

    // NOCOM(#sirver): this rpc should contain information about the created buffer.

    let broadcast_msg: plugin_buffer::NewBuffer =
        json::from_value(client.recv().unwrap()).unwrap();
    assert_eq!(plugin_buffer::NewBuffer {
        buffer_index: 0,
    }, broadcast_msg);
}
