use serde::json;
use support::{TestHarness, temporary_socket_name};
use switchboard::client::{self, RemoteProcedure, Client};
use switchboard::ipc::RpcResult;
use switchboard::server::Server;

#[test]
fn shutdown_server_with_clients_connected() {
    let socket_name = temporary_socket_name();
    let mut server = Server::launch(&socket_name);

    let _client = Client::connect(&socket_name);

    server.shutdown();
}

#[test]
fn shutdown_server_with_no_clients_connected() {
    let t = TestHarness::new();
    let _client = Client::connect(&t.socket_name);
}

#[test]
fn broadcast_works() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    let client2 = Client::connect(&t.socket_name);

    let test_msg = json::builder::ObjectBuilder::new()
        .insert("blub".into(), "blah")
        .unwrap();

    let rpc = client1.call("core.broadcast", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResult::success(()));

    let broadcast_msg = client1.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);

    let broadcast_msg = client2.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);
}

#[test]
fn register_function_and_call_it() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    let client2 = Client::connect(&t.socket_name);

    struct TestCall {
        client_handle: client::ClientHandle,
    };

    impl RemoteProcedure for TestCall {
        // NOCOM(#sirver): the client handle should be passed in.
        fn call(&mut self, args: json::Value) -> RpcResult {
            let rpc = self.client_handle.call("core.broadcast", &args);
            rpc.wait().unwrap()
        }
    }
    let client_handle = client1.client_handle();
    client1.register_function("testclient.test", Box::new(TestCall {
        client_handle: client_handle,
    }));

    let test_msg = json::builder::ObjectBuilder::new()
        .insert("blub".into(), "blah")
        .unwrap();

    let rpc = client2.call("testclient.test", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResult::success(()));

    let broadcast_msg = client1.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);

    let broadcast_msg = client2.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);
}
