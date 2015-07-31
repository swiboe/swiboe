extern crate switchboard;

use std::path::Path;

fn main() {
    let mut server = switchboard::server::Server::launch(&Path::new("/tmp/sb.socket"));
    server.wait_for_shutdown();
}
