#![cfg(not(test))]

#[macro_use]
extern crate clap;
extern crate swiboe;

use std::path::Path;

fn main() {
    let matches = clap::App::new("server")
        .about("Swiboe stand alone server.")
        .version(&crate_version!()[..])
        .arg(clap::Arg::with_name("SOCKET")
             .short("s")
             .long("socket")
             .help("Socket address on which to listen.")
             .required(true)
             .takes_value(true))
        .get_matches();

    let path = Path::new(matches.value_of("SOCKET").unwrap());
    let mut server = swiboe::server::Server::launch(path);
    server.wait_for_shutdown();
}
