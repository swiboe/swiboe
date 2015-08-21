use ::CallbackRpc;
use serde_json;
use std::env;
use std::path;
use std::sync;
use std::thread;
use support::TestHarness;
use swiboe::client;
use swiboe::rpc;
use swiboe::server::Server;
use uuid::Uuid;

fn temporary_socket_name() -> path::PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("{}.socket", Uuid::new_v4().to_string()));
    dir
}

fn as_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap()
}

#[test]
fn shutdown_server_with_clients_connected() {
    let socket_name = temporary_socket_name();
    let mut server = Server::launch(&socket_name);

    let _client = client::Client::connect(&socket_name).unwrap();

    server.shutdown();
}

#[test]
fn shutdown_server_with_no_clients_connected() {
    let t = TestHarness::new();
    let _client = client::Client::connect(&t.socket_name).unwrap();
}

struct TestCall {
    priority: u16,
    result: rpc::Result,
}

impl client::rpc::server::Rpc for TestCall {
    fn priority(&self) -> u16 { self.priority }
    fn call(&mut self, mut context: client::rpc::server::Context, _: serde_json::Value) {
        context.finish(self.result.clone()).unwrap();
    }
}


#[test]
fn new_rpc_simple() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name).unwrap();
    let client2 = client::Client::connect(&t.socket_name).unwrap();

    let test_msg: serde_json::Value = as_json(r#"{ "blub": "blah" }"#);

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

    let client1 = client::Client::connect(&t.socket_name).unwrap();
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));


    let client2 = client::Client::connect(&t.socket_name).unwrap();
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: rpc::Result::Ok(as_json(r#"{ "from": "client2" }"#)),
    }));

    let client3 = client::Client::connect(&t.socket_name).unwrap();
    let mut rpc = client3.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client2" }"#)), rpc.wait().unwrap());
}

#[test]
fn new_rpc_with_priority_first_does_not_handle() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name).unwrap();
    client1.new_rpc("test.test", Box::new(TestCall {
        priority: 100,
        result: rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));


    let client2 = client::Client::connect(&t.socket_name).unwrap();
    client2.new_rpc("test.test", Box::new(TestCall {
        priority: 50,
        result: rpc::Result::NotHandled,
    }));

    let client3 = client::Client::connect(&t.socket_name).unwrap();
    let mut rpc = client3.call("test.test", &as_json(r#"{}"#));
    assert_eq!(rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)), rpc.wait().unwrap());
}

#[test]
#[should_panic]
fn rpc_not_calling_finish() {
    let t = TestHarness::new();

    let client1 = client::Client::connect(&t.socket_name).unwrap();
    client1.new_rpc("test.test", Box::new(CallbackRpc {
        priority: 100,
        callback: |_, _| {},
    }));

    let client2 = client::Client::connect(&t.socket_name).unwrap();
    // TODO(sirver): This should timeout, but that is not implemented yet.
    client2.call("test.test", &as_json(r#"{}"#));

    // Should be plenty to have test.test being handled.
    thread::sleep_ms(500);
}


#[test]
fn client_disconnects_should_not_stop_handling_of_rpcs() {
    let t = TestHarness::new();

    let client0 = client::Client::connect(&t.socket_name).unwrap();
    client0.new_rpc("test.test", Box::new(TestCall {
            priority: 100, result: rpc::Result::NotHandled,
    }));

    let client1 = client::Client::connect(&t.socket_name).unwrap();
    client1.new_rpc("test.test", Box::new(TestCall {
            priority: 101, result:
                rpc::Result::Ok(as_json(r#"{ "from": "client1" }"#)),
    }));

    let client2 = client::Client::connect(&t.socket_name).unwrap();
    client2.new_rpc("test.test", Box::new(TestCall {
            priority: 102, result: rpc::Result::NotHandled,
    }));

    let client3 = client::Client::connect(&t.socket_name).unwrap();
    client3.new_rpc("test.test", Box::new(TestCall {
            priority: 103, result:
                rpc::Result::Ok(as_json(r#"{ "from": "client3" }"#)),
    }));

    let client = client::Client::connect(&t.socket_name).unwrap();

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

    let client = client::Client::connect(&t.socket_name).unwrap();
    let mut rpc = client.call("not_existing", &as_json("{}"));
    assert_eq!(rpc::Result::Err(rpc::Error {
        kind: rpc::ErrorKind::UnknownRpc,
        details: None,
    }), rpc.wait().unwrap());
}

#[test]
fn call_streaming_rpc_simple() {
    // NOCOM(#sirver): test for next_result on non streaming rpc
    let t = TestHarness::new();

    let streaming_client = client::Client::connect(&t.socket_name).unwrap();
    streaming_client.new_rpc("test.test", Box::new(CallbackRpc {
        priority: 50,
        callback: |mut context: client::rpc::server::Context, _| {
            thread::spawn(move || {
                context.update(&as_json(r#"{ "msg": "one" }"#)).unwrap();
                context.update(&as_json(r#"{ "msg": "two" }"#)).unwrap();
                context.update(&as_json(r#"{ "msg": "three" }"#)).unwrap();
                context.finish(rpc::Result::success(&as_json(r#"{ "foo": "blah" }"#))).unwrap();
            });
        },
    }));

    let client = client::Client::connect(&t.socket_name).unwrap();
    let mut rpc = client.call("test.test", &as_json("{}"));

    assert_eq!(as_json(r#"{ "msg": "one" }"#), rpc.recv().unwrap().unwrap());
    assert_eq!(as_json(r#"{ "msg": "two" }"#), rpc.recv().unwrap().unwrap());
    assert_eq!(as_json(r#"{ "msg": "three" }"#), rpc.recv().unwrap().unwrap());
    assert_eq!(rpc::Result::success(as_json(r#"{ "foo": "blah" }"#)), rpc.wait().unwrap());
}

#[test]
fn call_streaming_rpc_cancelled() {
    let cancelled = sync::Arc::new(sync::Mutex::new(false));

    let t = TestHarness::new();
    let streaming_client = client::Client::connect(&t.socket_name).unwrap();
    streaming_client.new_rpc("test.test", Box::new(CallbackRpc {
        priority: 50,
        callback: |mut context: client::rpc::server::Context, _| {
            let cancelled = cancelled.clone();
            thread::spawn(move || {
                let mut count = 0;
                // NOCOM(#sirver): cancelled? grep for that.
                while !context.cancelled() {
                    context.update(
                        &as_json(&format!(r#"{{ "value": "{}" }}"#, count))).unwrap();
                    thread::sleep_ms(10);
                    count += 1
                }
                assert!(context.finish(rpc::Result::success(
                            &as_json(r#"{ "foo": "blah" }"#))).is_err());
                let mut cancelled = cancelled.lock().unwrap();
                *cancelled = true;
            });
        },
    }));

    let client = client::Client::connect(&t.socket_name).unwrap();
    let mut rpc = client.call("test.test", &as_json("{}"));

    assert_eq!(as_json(r#"{ "value": "0" }"#), rpc.recv().unwrap().unwrap());
    assert_eq!(as_json(r#"{ "value": "1" }"#), rpc.recv().unwrap().unwrap());
    assert_eq!(as_json(r#"{ "value": "2" }"#), rpc.recv().unwrap().unwrap());
    assert_eq!(as_json(r#"{ "value": "3" }"#), rpc.recv().unwrap().unwrap());

    rpc.cancel().unwrap();

    // Wait for the server thread to end. If anything went wrong this will sit forever.
    loop {
        let cancelled = cancelled.lock().unwrap();
        if *cancelled == true {
            break;
        }
    }
}
