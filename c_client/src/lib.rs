// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(cstr_memory)]

extern crate libc;

use libc::c_char;
use std::ffi::{CStr, CString};
use std::str;

#[no_mangle]
pub extern "C" fn hello(c_buf: *const libc::c_char) {
    let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
    let buf: &[u8] = c_str.to_bytes();
    let str_slice: &str = str::from_utf8(buf).unwrap();
    // let str_buf: String = str_slice.to_owned();  // if necessary
    println!("#sirver str_slice(): {:#?}", str_slice);
}

#[no_mangle]
pub extern "C" fn hello1(cb: extern fn(i32, *mut *const libc::c_char)) {
    println!("In rust!");
    let mut ptr: *const libc::c_char;
    let c_buf = cb(42, &mut ptr);
    let c_str = unsafe { CStr::from_ptr(c_buf) };
    let buf: &[u8] = c_str.to_bytes();
    let str_slice: &str = str::from_utf8(buf).unwrap();
    println!("Back in rust: {}", str_slice);
}
