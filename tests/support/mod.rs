use std::env;
use std::path::{PathBuf};
use switchboard::server::Server;
use uuid::Uuid;

pub struct TestServer<'a> {
    server: Option<Server<'a>>,
}

impl<'a> TestServer<'a> {
    pub fn new() -> (Self, PathBuf) {
        let socket_name = temporary_socket_name();
        let server = Server::launch(&socket_name);

        (TestServer { server: Some(server), }, socket_name)
    }
}

impl<'a> Drop for TestServer<'a> {
    fn drop(&mut self) {
        self.server.take().unwrap().shutdown();
    }
}

pub fn temporary_socket_name() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("{}.socket", Uuid::new_v4().to_string()));
    dir
}
