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

fn as_json(s: &str) -> json::Value {
    json::from_str(s).unwrap()
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
fn new_rpc_simple() {
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
        result: RpcResult::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));


    let client2 = Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: RpcResult::Ok(as_json(r#"{ "from": "client2" }"#)),
    }));

    let client3 = Client::connect(&t.socket_name);
    let rpc = client3.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(as_json(r#"{ "from": "client2" }"#)), rpc.wait().unwrap());
}

#[test]
fn new_rpc_with_priority_first_does_not_handle() {
    let t = TestHarness::new();

    let client1 = Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: RpcResult::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));


    let client2 = Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: RpcResult::NotHandled,
    }));

    let client3 = Client::connect(&t.socket_name);
    let rpc = client3.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(as_json(r#"{ "from": "client1" }"#)), rpc.wait().unwrap());
}

#[test]
fn client_disconnects_should_not_stop_handling_of_rpcs() {
    let t = TestHarness::new();

    let client0 = Client::connect(&t.socket_name);
    client0.new_rpc("test.test", Box::new(TestCall {
            priority: 100, result: RpcResult::NotHandled,
    }));

    let client1 = Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
            priority: 101, result:
                RpcResult::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));

    let client2 = Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
            priority: 102, result: RpcResult::NotHandled,
    }));

    let client3 = Client::connect(&t.socket_name);
    client3.new_rpc("test.test", Box::new(TestCall {
            priority: 103, result:
                RpcResult::Ok(as_json(r#"{ "from": "client3" }"#)),
    }));

    let client = Client::connect(&t.socket_name);

    let rpc = client.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(as_json(r#"{ "from": "client1" }"#)), rpc.wait().unwrap());

    drop(client1); // clients: 0 2 3
    let rpc = client.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(as_json(r#"{ "from": "client3" }"#)), rpc.wait().unwrap());

    drop(client0); // clients: 2 3
    let rpc = client.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::Ok(as_json(r#"{ "from": "client3" }"#)), rpc.wait().unwrap());

    drop(client3); // clients: 2
    let rpc = client.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());
    assert_eq!(RpcResult::NotHandled, rpc.wait().unwrap());

    drop(client2); // clients:

    let rpc = client.call("test.test", &json::from_str::<json::Value>(r#"{}"#).unwrap());

    // NOCOM(#sirver): now, nobody can handle this RPC anymore - we should return Unknown RPC.
    // Since we do not properly clean up after disconnects of clients, it does not work yet.
    // assert_eq!(RpcResult::Err(RpcError {
        // kind: RpcErrorKind::UnknownRpc,
        // details: None,
    // }), rpc.wait().unwrap());
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
