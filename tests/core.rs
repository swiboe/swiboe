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

    let test_msg: json::Value = json::from_str(r#"{ "blub": "blah" }"#).unwrap();

    let rpc = client1.call("core.broadcast", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResult::success(()));

    let broadcast_msg = client1.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);

    let broadcast_msg = client2.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);
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
fn register_function() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    let client2 = Client::connect(&t.socket_name);

    let test_msg: json::Value = json::from_str(r#"{ "blub": "blah" }"#).unwrap();

    client1.register_function("test.test", Box::new(TestCall {
        priority: 0,
        result: RpcResult::Ok(test_msg.clone()),
    }));

    let rpc = client2.call("test.test", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResult::Ok(test_msg));
}

#[test]
fn register_function_with_priority() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    client1.register_function("test.test", Box::new(TestCall {
        priority: 100,
        result: RpcResult::Ok(json::from_str(r#"{ "from": "client1" }"#).unwrap()),
    }));


    let client2 = Client::connect(&t.socket_name);
    client2.register_function("test.test", Box::new(TestCall {
        priority: 50,
        result: RpcResult::Ok(json::from_str(r#"{ "from": "client2" }"#).unwrap()),
    }));

    let client3 = Client::connect(&t.socket_name);
    let rpc = client3.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(json::from_str(r#"{ "from": "client2" }"#).unwrap()), rpc.wait().unwrap());
}

#[test]
fn register_function_with_priority_first_does_not_handle() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    client1.register_function("test.test", Box::new(TestCall {
        priority: 100,
        result: RpcResult::Ok(json::from_str(r#"{ "from": "client1" }"#).unwrap()),
    }));


    let client2 = Client::connect(&t.socket_name);
    client2.register_function("test.test", Box::new(TestCall {
        priority: 50,
        result: RpcResult::NotHandled,
    }));

    let client3 = Client::connect(&t.socket_name);
    let rpc = client3.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(json::from_str(r#"{ "from": "client1" }"#).unwrap()), rpc.wait().unwrap());
}