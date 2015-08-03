extern crate serde;
extern crate switchboard;
extern crate uuid;

use serde::json;
use std::env;
use std::path::{PathBuf};
use switchboard::client::{self, RemoteProcedure, Client};
use switchboard::ipc::RpcResultKind;
use switchboard::plugin_buffer;
use switchboard::server::Server;
use switchboard::testing::{TestServer, temporary_socket_name};
use uuid::Uuid;

// NOCOM(#sirver): use the name switchboard everywhere.

#[test]
fn shutdown_server_with_clients_connected() {
    let socket_name = temporary_socket_name();
    let mut server = Server::launch(&socket_name);

    let _client = Client::connect(&socket_name);

    server.shutdown();
}

#[test]
fn shutdown_server_with_no_clients_connected() {
    let (_server, socket_name) = TestServer::new();
    let _client = Client::connect(&socket_name);
}

#[test]
fn broadcast_works() {
    let (_server, socket_name) = TestServer::new();

    let client1 = Client::connect(&socket_name);
    let client2 = Client::connect(&socket_name);

    let test_msg = json::builder::ObjectBuilder::new()
        .insert("blub".into(), "blah")
        .unwrap();

    let rpc = client1.call("core.broadcast", &test_msg);
    assert_eq!(rpc.wait().unwrap(), RpcResultKind::Ok);

    let broadcast_msg = client1.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);

    let broadcast_msg = client2.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);
}

#[test]
fn register_function_and_call_it() {
    let (_server, socket_name) = TestServer::new();

    let client1 = Client::connect(&socket_name);
    let client2 = Client::connect(&socket_name);

    struct TestCall {
        client_handle: client::ClientHandle,
    };

    impl RemoteProcedure for TestCall {
        // NOCOM(#sirver): the client handle should be passed in.
        fn call(&mut self, args: json::Value) -> RpcResultKind {
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
    assert_eq!(rpc.wait().unwrap(), RpcResultKind::Ok);

    let broadcast_msg = client1.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);

    let broadcast_msg = client2.recv().unwrap();
    assert_eq!(test_msg, broadcast_msg);
}

// NOCOM(#sirver): test is needed.
// #[test]
// fn waiting_for_call_does_not_mean_you_miss_data() {
    // let (_server, socket_name) = TestServer::new();

    // let client1 = Client::connect(&socket_name);
    // let client2 = Client::connect(&socket_name);

    // let test_msg = json::builder::ObjectBuilder::new()
        // .insert("blub".into(), "blah")
        // .unwrap();

    // let rpc = client1.call("core.broadcast", &test_msg);
    // assert_eq!(rpc.wait().unwrap(), RpcResultKind::Ok);

    // let broadcast_msg = client1.recv().unwrap();
    // assert_eq!(test_msg, broadcast_msg);

    // let broadcast_msg = client2.recv().unwrap();
    // assert_eq!(test_msg, broadcast_msg);
// }
