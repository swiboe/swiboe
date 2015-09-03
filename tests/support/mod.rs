use std::path::PathBuf;
use swiboe::server::Server;
use tempdir::TempDir;

pub struct TestHarness {
    server: Option<Server>,
    pub socket_name: PathBuf,
    pub temp_directory: TempDir,
}

impl TestHarness {
    pub fn new() -> Self {
        let temp_directory = TempDir::new("swiboe").unwrap();

        let mut socket_name = temp_directory.path().to_path_buf();
        socket_name.push("_socket");

        let server = Server::launch(&socket_name, &[]);

        TestHarness {
            server: Some(server),
            socket_name: socket_name,
            temp_directory: temp_directory,
        }
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        self.server.take().unwrap().shutdown();
    }
}
