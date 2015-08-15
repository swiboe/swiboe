use serde::json;
use std::env;
use std::path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use super::CallbackProcedure;
use support::TestHarness;
use switchboard::client;
use switchboard::rpc;
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

    let _client = client::Client::connect(&socket_name);

    server.shutdown();
}

#[test]
fn shutdown_server_with_no_clients_connected() {
    let t = TestHarness::new();
    let _client = client::Client::connect(&t.socket_name);
}

struct TestCall {
    priority: u16,
    result: rpc::Result,
}

impl client::RemoteProcedure for TestCall {
    fn priority(&self) -> u16 { self.priority }
    fn call(&mut self, mut sender: client::RpcSender, _: json::Value) {
        sender.finish(self.result.clone());
    }
}


#[test]
fn new_rpc_simple() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name);
    let client2 = client::Client::connect(&t.socket_name);

    let test_msg: json::Value = as_json(r#"{ "blub": "blah" }"#);

    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 0,
        result: rpc::Result::Ok(test_msg.clone()),
    }));

    let mut rpc = client2.call("test.test", &test_msg);
    assert_eq!(rpc.wait().unwrap(), rpc::Result::Ok(test_msg));
}

#[test]
fn new_rpc_with_priority() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));


    let client2 = client::Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: rpc::Result::Ok(as_json(r#"{ "from": "client2" }"#)),
    }));

    let client3 = client::Client::connect(&t.socket_name);
    let mut rpc = client3.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client2" }"#)), rpc.wait().unwrap());
}

#[test]
fn new_rpc_with_priority_first_does_not_handle() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));


    let client2 = client::Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: rpc::Result::NotHandled,
    }));

    let client3 = client::Client::connect(&t.socket_name);
    let mut rpc = client3.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)), rpc.wait().unwrap());
}

#[test]
#[should_panic]
fn rpc_not_calling_finish() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(CallbackProcedure {
        priority: 100,
        callback: |sender, args| {},
    }));

    let client2 = client::Client::connect(&t.socket_name);
    // TODO(sirver): This should timeout, but that is not implemented yet.
    let mut rpc = client2.call("test.test", &as_json(r#"{}"#));

    // Should be plenty to have test.test being handled.
    thread::sleep_ms(500);
}


#[test]
fn client_disconnects_should_not_stop_handling_of_rpcs() {
    let t = TestHarness::new();

    let client0 = client::Client::connect(&t.socket_name);
    client0.new_rpc("test.test", Box::new(TestCall {
            priority: 100, result: rpc::Result::NotHandled,
    }));

    let client1 = client::Client::connect(&t.socket_name);
    client1.new_rpc("test.test", Box::new(TestCall {
            priority: 101, result:
                rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));

    let client2 = client::Client::connect(&t.socket_name);
    client2.new_rpc("test.test", Box::new(TestCall {
            priority: 102, result: rpc::Result::NotHandled,
    }));

    let client3 = client::Client::connect(&t.socket_name);
    client3.new_rpc("test.test", Box::new(TestCall {
            priority: 103, result:
                rpc::Result::Ok(as_json(r#"{ "from": "client3" }"#)),
    }));

    let client = client::Client::connect(&t.socket_name);

    let mut rpc = client.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)), rpc.wait().unwrap());

    drop(client1); // clients: 0 2 3
    let mut rpc = client.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client3" }"#)), rpc.wait().unwrap());

    drop(client0); // clients: 2 3
    let mut rpc = client.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client3" }"#)), rpc.wait().unwrap());

    drop(client3); // clients: 2
    let mut rpc = client.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::NotHandled, rpc.wait().unwrap());

    drop(client2); // clients:

    let mut rpc = client.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Err(rpc::Error {
        kind: rpc::ErrorKind::UnknownRpc,
        details: None,
    }), rpc.wait().unwrap());
}

#[test]
fn call_not_existing_rpc() {
    let t = TestHarness::new();

    let client = client::Client::connect(&t.socket_name);
    let mut rpc = client.call("not_existing", &as_json("{}"));
    assert_eq!(rpc::Result::Err(rpc::Error {
        kind: rpc::ErrorKind::UnknownRpc,
        details: None,
    }), rpc.wait().unwrap());
}

#[test]
fn call_streaming_rpc_simple() {
    let callback_called = AtomicBool::new(false);
    {
        // NOCOM(#sirver): test for next_result on non streaming rpc
        let t = TestHarness::new();

        let streaming_client = client::Client::connect(&t.socket_name);
        streaming_client.new_rpc("test.test", Box::new(CallbackProcedure {
            priority: 50,
            callback: |mut rpc_sender: client::RpcSender, _| {
                thread::spawn(move || {
                    rpc_sender.update(&as_json(r#"{ "msg": "one" }"#));
                    rpc_sender.update(&as_json(r#"{ "msg": "two" }"#));
                    rpc_sender.update(&as_json(r#"{ "msg": "three" }"#));
                    rpc_sender.finish(rpc::Result::success(&as_json(r#"{ "foo": "blah" }"#)));
                });
            },
        }));

        let client = client::Client::connect(&t.socket_name);
        let mut rpc = client.call("test.test", &as_json("{}"));

        assert_eq!(as_json(r#"{ "msg": "one" }"#), rpc.recv().unwrap().unwrap());
        assert_eq!(as_json(r#"{ "msg": "two" }"#), rpc.recv().unwrap().unwrap());
        assert_eq!(as_json(r#"{ "msg": "three" }"#), rpc.recv().unwrap().unwrap());
        assert_eq!(rpc::Result::success(as_json(r#"{ "foo": "blah" }"#)), rpc.wait().unwrap());
    }
    assert!(!callback_called.load(Ordering::Relaxed));
}
