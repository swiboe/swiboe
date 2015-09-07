// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(cstr_memory)]

extern crate libc;
extern crate swiboe;

use libc::c_char;
use std::ffi::{CStr, CString};
use std::mem;
use std::path;
use std::str;
use swiboe::client;

// TODO(sirver): this always makes a copy, even though it might not be needed.
fn c_str_to_string(c_buf: *const c_char) -> String {
    let c_str = unsafe { CStr::from_ptr(c_buf) };
    let buf: &[u8] = c_str.to_bytes();
    // NOCOM(#sirver): do not unwrap, do error handling.
    str::from_utf8(buf).unwrap().into()
}

// TODO(sirver): This crashes if the function is called connect.
#[no_mangle]
pub extern "C" fn create_client(socket_name: *const c_char) -> *mut client::Client {
    let socket_name = c_str_to_string(socket_name);
    let socket_name_path = path::Path::new(&socket_name);

    let client = Box::new(
        // NOCOM(#sirver): error handling
        client::Client::connect(socket_name_path).unwrap(),
    );

    unsafe { mem::transmute(client) }
}

#[no_mangle]
pub extern "C" fn disconnect(client: *mut client::Client) {
    unsafe {
        let _: Box<client::Client> = mem::transmute(client);
    }
}
