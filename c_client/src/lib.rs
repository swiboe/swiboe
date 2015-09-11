// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

#![feature(cstr_memory)]
#![feature(result_expect)]

extern crate libc;
extern crate serde;
extern crate serde_json;
extern crate swiboe;

use libc::c_char;
use std::ffi::{CStr, CString};
use std::mem;
use std::path;
use std::str;
use swiboe::client;
use swiboe::rpc;

// TODO(sirver): this always makes a copy, even though it might not be needed.
fn c_str_to_string(c_buf: *const c_char) -> String {
    let c_str = unsafe { CStr::from_ptr(c_buf) };
    let buf: &[u8] = c_str.to_bytes();
    // NOCOM(#sirver): do not unwrap, do error handling.
    str::from_utf8(buf).unwrap().into()
}

#[no_mangle]
pub extern "C" fn swiboe_connect(socket_name: *const c_char) -> *mut client::Client {
    let socket_name = c_str_to_string(socket_name);
    let socket_name_path = path::Path::new(&socket_name);

    let client = Box::new(
        // NOCOM(#sirver): error handling
        client::Client::connect(socket_name_path).unwrap(),
    );

    unsafe { mem::transmute(client) }
}

#[no_mangle]
pub extern "C" fn swiboe_disconnect(client: *mut client::Client) {
    unsafe {
        let _: Box<client::Client> = mem::transmute(client);
    }
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_ok(c_buf: *const c_char) -> *const rpc::Result {
    let json_str = c_str_to_string(c_buf);

    unsafe {
        mem::transmute(Box::new(rpc::Result::success(&json_str)))
    }
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_error(c_buf_error_name: *const c_char, c_buf_details: *const c_char) -> *const rpc::Result {
    let error_name = c_str_to_string(c_buf_error_name);
    let details = if c_buf_details.is_null() {
        None
    } else {
        let json_str = c_str_to_string(c_buf_details);
        Some(serde_json::from_str(&json_str).expect("swiboe_rpc_error: 'details' not valid json."))
    };

    let err = rpc::Error {
        kind: rpc::ErrorKind::from_str(&error_name),
        details: details,
    };

    unsafe {
        mem::transmute(Box::new(rpc::Result::Err(err)))
    }
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_not_handled() -> *const rpc::Result {
    unsafe {
        mem::transmute(Box::new(rpc::Result::NotHandled))
    }
}

#[no_mangle]
pub extern "C" fn swiboe_client_call_rpc(client: *const client::Client,
                                         rpc_name: *const c_char,
                                         args: *const c_char) -> *mut client::rpc::client::Context {
    let client: &mut client::Client = unsafe {
        mem::transmute(client)
    };

    let rpc_name = c_str_to_string(rpc_name);
    let args = if args.is_null() {
        serde_json::Value::Null
    } else {
        let json_str = c_str_to_string(args);
        serde_json::from_str(&json_str).expect("swiboe_client_call_rpc: 'args' not valid json.")
    };

    unsafe {
        mem::transmute(Box::new(client.call(&rpc_name, &args)))
    }
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_context_wait(context: *mut client::rpc::client::Context) {
    let mut context: Box<client::rpc::client::Context> = unsafe {
        mem::transmute(context)
    };

    // NOCOM(#sirver): return results to python
    let result = context.wait();
}


pub type CCallback = extern fn(*const c_char) -> *const rpc::Result;
struct CallbackRpc {
    priority: u16,
    callback: CCallback,
}

impl client::rpc::server::Rpc for CallbackRpc {
    fn priority(&self) -> u16 { self.priority }

    fn call(&self,
            mut context: client::rpc::server::Context,
            args: serde_json::Value) {
        let args_str = serde_json::to_string(&args).unwrap();
        let c_str = CString::new(args_str).expect("JSON contained zero byte");
        let result: Box<rpc::Result> = unsafe {
            let result = (self.callback)(c_str.as_ptr());
            let res = mem::transmute(result);
            res
        };
        context.finish(*result).unwrap();
    }
}

// NOCOM(#sirver): add error handling.
#[no_mangle]
pub extern "C" fn swiboe_new_rpc(client: *mut client::Client,
                                 rpc_name: *const c_char,
                                 priority: libc::uint16_t,
                                 callback: CCallback
                                 ) {
    let client: &mut client::Client = unsafe {
        mem::transmute(client)
    };

    let rpc = Box::new(CallbackRpc {
        priority: priority,
        callback: callback,
    });

    let rpc_name= c_str_to_string(rpc_name);
    client.new_rpc(&rpc_name, rpc);
}
