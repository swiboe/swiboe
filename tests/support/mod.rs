use std::path::PathBuf;
use switchboard::server::Server;
use tempdir::TempDir;

pub struct TestHarness<'a> {
    server: Option<Server<'a>>,
    pub socket_name: PathBuf,
    pub temp_directory: TempDir,
}

impl<'a> TestHarness<'a> {
    pub fn new() -> Self {
        let temp_directory = TempDir::new("switchboard").unwrap();

        let mut socket_name = temp_directory.path().to_path_buf();
        socket_name.push("_socket");

        let server = Server::launch(&socket_name);

        TestHarness {
            server: Some(server),
            socket_name: socket_name,
            temp_directory: temp_directory,
        }
    }
}

impl<'a> Drop for TestHarness<'a> {
    fn drop(&mut self) {
        self.server.take().unwrap().shutdown();
    }
}
