// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![cfg(not(test))]

#[macro_use]
extern crate clap;
extern crate serde;
extern crate serde_json;
extern crate swiboe;
extern crate tempdir;

use std::path;
use swiboe::client::Client;
use swiboe::plugin_buffer;
use swiboe::rpc;

fn main() {
    let matches = clap::App::new("term_gui")
        .about("Terminal client for Swiboe")
        .version(&crate_version!()[..])
        .arg(clap::Arg::with_name("SOCKET")
             .short("s")
             .long("socket")
             .help("Socket at which the master listens.")
             .required(true)
             .takes_value(true))
        .get_matches();

    let path = path::Path::new(matches.value_of("SOCKET").unwrap());
    let active_client = Client::connect_unix(path).unwrap();

    loop {
        let new_response: plugin_buffer::NewResponse = match active_client.call(
            "buffer.new", &plugin_buffer::NewRequest {
                content: Some("bli\nbla\nblub".into()),
            }).wait().unwrap()
        {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };

        let _: plugin_buffer::DeleteResponse = match active_client.call(
            "buffer.delete", &plugin_buffer::DeleteRequest {
                buffer_index: new_response.buffer_index
            }).wait().unwrap() {
            rpc::Result::Ok(value) => serde_json::from_value(value).unwrap(),
            err => panic!("{:?}", err),
        };
    }
}
