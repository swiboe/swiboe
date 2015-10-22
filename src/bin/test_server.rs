// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![cfg(not(test))]

#[macro_use]
extern crate clap;
extern crate swiboe;

use swiboe::testing::TestHarness;

use std::path::Path;

fn main() {
    let matches = clap::App::new("server")
        .about("Swiboe stand alone server for testing.")
        .version(&crate_version!()[..])
        .get_matches();

    let mut test_harness = TestHarness::new();

    // NOCOM(#sirver): instead, print out json file name that contains more structured data.
    println!("{}", test_harness.temp_directory.path().to_str().unwrap());

    // NOCOM(#sirver): this only exits on Ctrl-C, but then has no chance to clean up. how to deal
    // with that?
    test_harness.wait_for_shutdown();
}
