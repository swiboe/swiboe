use serde::json;
use std::env;
use std::path;
use support::TestHarness;
use switchboard::client::{RemoteProcedure, Client};
use switchboard::ipc::RpcResult;
use switchboard::server::Server;
use uuid::Uuid;

fn temporary_socket_name() -> path::PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("{}.socket", Uuid::new_v4().to_string()));
    dir
}

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

    let test_msg: json::Value = json::from_str(r#"{ "blub": "blah" } "#).unwrap();

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

    struct TestCall;
    impl RemoteProcedure for TestCall {
        fn call(&mut self, args: json::Value) -> RpcResult {
            RpcResult::Ok(args)
        }
    }

    client1.register_function("testclient.test", Box::new(TestCall));

    let test_msg = json::from_str(r#"{ "blub": "blah" } "#).unwrap();

    let rpc = client2.call("testclient.test", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResult::Ok(test_msg));
}
