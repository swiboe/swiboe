use std::path::PathBuf;
use std::io::{Write};
use switchboard::server::Server;
use tempdir::TempDir;
use std::fs;

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

    pub fn new_file(&self, name: &str, content: &str) -> PathBuf {
        let mut file_name = self.temp_directory.path().to_path_buf();
        file_name.push(name);

        let mut f = fs::File::create(&file_name).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        file_name
    }
}

impl<'a> Drop for TestHarness<'a> {
    fn drop(&mut self) {
        self.server.take().unwrap().shutdown();
    }
}
