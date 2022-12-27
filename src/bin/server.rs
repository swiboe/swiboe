// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![cfg(not(test))]

#[macro_use]
extern crate clap;
extern crate swiboe;

use std::path::Path;

fn main() {
    let matches = clap::App::new("server")
        .about("Swiboe stand alone server.")
        .version(&crate_version!()[..])
        .arg(
            clap::Arg::with_name("SOCKET")
                .short("s")
                .long("socket")
                .help("Socket address on which to listen.")
                .required(true)
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("LISTEN")
                .short("l")
                .long("listen")
                .help(
                    "IP address to listen on, e.g. 0.0.0.0:12345 to listen on all network \
                   interfaces.",
                )
                .takes_value(true),
        )
        .get_matches();

    let path = Path::new(matches.value_of("SOCKET").unwrap());
    let ips = if let Some(addr) = matches.value_of("LISTEN") {
        vec![addr.into()]
    } else {
        Vec::new()
    };

    let mut server = swiboe::server::Server::launch(path, &ips).unwrap();
    server.wait_for_shutdown();
}
