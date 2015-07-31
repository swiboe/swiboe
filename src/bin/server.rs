extern crate switchboard;

use std::path::Path;

fn main() {
    let mut s = switchboard::server::Server::launch(&Path::new("/tmp/sb.socket"));
    s.wait_for_shutdown();
}
