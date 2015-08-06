use serde::json;
use std::env;
use std::path;
use support::TestHarness;
use switchboard::client::{RemoteProcedure, Client};
use switchboard::ipc::{RpcErrorKind, RpcResult, RpcError};
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

struct TestCall {
    priority: u16,
    result: RpcResult,
}

impl RemoteProcedure for TestCall {
    fn priority(&self) -> u16 { self.priority }
    fn call(&mut self, _: json::Value) -> RpcResult { self.result.clone() }
}


#[test]
fn new_rpc() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    let client2 = Client::connect(&t.socket_name);

    let test_msg: json::Value = json::from_str(r#"{ "blub": "blah" }"#).unwrap();

    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 0,
        result: RpcResult::Ok(test_msg.clone()),
    }));

    let rpc = client2.call("test.test", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResult::Ok(test_msg));
}

#[test]
fn new_rpc_with_priority() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: RpcResult::Ok(json::from_str(r#"{ "from": "client1" }"#).unwrap()),
    }));


    let client2 = Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: RpcResult::Ok(json::from_str(r#"{ "from": "client2" }"#).unwrap()),
    }));

    let client3 = Client::connect(&t.socket_name);
    let rpc = client3.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(json::from_str(r#"{ "from": "client2" }"#).unwrap()), rpc.wait().unwrap());
}

#[test]
fn new_rpc_with_priority_first_does_not_handle() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: RpcResult::Ok(json::from_str(r#"{ "from": "client1" }"#).unwrap()),
    }));


    let client2 = Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: RpcResult::NotHandled,
    }));

    let client3 = Client::connect(&t.socket_name);
    let rpc = client3.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(json::from_str(r#"{ "from": "client1" }"#).unwrap()), rpc.wait().unwrap());
}

#[test]
fn call_not_existing_rpc() {
    let t = TestHarness::new();

    let client = Client::connect(&t.socket_name);
    let rpc = client.call("not_existing", &json::from_str::<json::Value>("{}").unwrap());
    assert_eq!(RpcResult::Err(RpcError {
        kind: RpcErrorKind::UnknownRpc,
        details: None,
    }), rpc.wait().unwrap());
}
