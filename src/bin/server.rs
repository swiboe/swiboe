extern crate s;

use std::path::Path;

fn main() {
    let mut s = s::server::Server::launch(&Path::new("/tmp/sb.socket"));
    s.wait_for_shutdown();
}
