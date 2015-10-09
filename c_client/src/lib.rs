// Copyright (c) The Swiboe development team. All rights reserved.
// Licensed under the Apache License, Version 2.0. See LICENSE.txt
// in the project root for license information.

extern crate libc;
extern crate serde;
extern crate serde_json;
extern crate swiboe;

use libc::c_char;
use std::ffi::{CStr, CString};
use std::mem;
use std::path;
use std::ptr;
use std::str;
use swiboe::{client, rpc};

pub type CApiResult = libc::int32_t;

const SUCCESS: CApiResult = 0;
const ERR_DISCONNECTED: CApiResult = 1;
const ERR_IO: CApiResult = 2;
const ERR_JSON_PARSING: CApiResult = 3;
const ERR_RPC_DONE: CApiResult = 4;
const ERR_INVALID_UTF8: CApiResult = 5;

macro_rules! try_capi {
    ($expr:expr) => (match $expr {
        ::std::result::Result::Ok(val) => val,
        ::std::result::Result::Err(swiboe_error) => {
            return match swiboe_error {
                swiboe::Error::Disconnected => ERR_DISCONNECTED,
                swiboe::Error::Io(_) => ERR_IO,
                swiboe::Error::JsonParsing(_) => ERR_JSON_PARSING,
                swiboe::Error::RpcDone => ERR_RPC_DONE,
                swiboe::Error::InvalidUtf8 => ERR_INVALID_UTF8,
            }
        }
    })
}

/// Converts a buffer we got passed from the API user or dies if the input is invalid.
fn to_str_or_die(c_str: &CStr) -> &str {
    str::from_utf8(c_str.to_bytes())
        .expect("argument was not a valid UTF-8 encoded string.")
}

/// Converts a buffer we got passed from the API user or dies if the input is invalid.
fn to_json_or_die(c_str: &CStr) -> serde_json::Value {
    let json_str = to_str_or_die(c_str);
    let json_value: serde_json::Value = serde_json::from_str(json_str)
        .expect("argument was not valid JSON.");
    json_value
}

#[no_mangle]
pub extern "C" fn swiboe_connect(socket_name: *const c_char, client: *mut *const client::Client) -> CApiResult {
    let socket_name_cstr = unsafe {
        CStr::from_ptr(socket_name)
    };

    let socket_name = to_str_or_die(&socket_name_cstr);
    let socket_name_path = path::Path::new(socket_name);

    let client_box = Box::new(
        try_capi!(client::Client::connect_unix(socket_name_path))
    );
    unsafe {
        *client = mem::transmute(client_box);
    }
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_disconnect(client: *mut client::Client) -> CApiResult {
    unsafe {
        let _: Box<client::Client> = mem::transmute(client);
    }
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_server_context_finish(context: *mut client::rpc::server::Context, rpc_result: *const rpc::Result) -> CApiResult {
    let mut context: Box<client::rpc::server::Context> = unsafe {
         mem::transmute(context)
    };
    let result: Box<rpc::Result> = unsafe {
         mem::transmute(rpc_result)
    };
    try_capi!(context.finish(*result));
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_server_context_update(context: *mut client::rpc::server::Context, json_c_buf: *const c_char) -> CApiResult {
    let mut context: &mut client::rpc::server::Context = unsafe {
         mem::transmute(context)
    };

    let json_c_str = unsafe {
        CStr::from_ptr(json_c_buf)
    };
    let json_value = to_json_or_die(&json_c_str);

    try_capi!(context.update(&json_value));
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_ok(c_buf: *const c_char) -> *const rpc::Result {
    let json_c_str = unsafe {
        CStr::from_ptr(c_buf)
    };
    let json_value = to_json_or_die(&json_c_str);
    unsafe {
        mem::transmute(Box::new(rpc::Result::Ok(json_value)))
    }
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_error(c_buf_error_name: *const c_char, c_buf_details: *const c_char) -> *const rpc::Result {
    let c_str_error_name = unsafe {
        CStr::from_ptr(c_buf_error_name)
    };
    let error_name = to_str_or_die(&c_str_error_name);

    let details = if c_buf_details.is_null() {
        None
    } else {
        let c_str_details = unsafe {
            CStr::from_ptr(c_buf_details)
        };
        Some(to_json_or_die(&c_str_details))
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

fn call<T: client::RpcCaller>(context: &mut T, rpc_name: *const c_char, args: *const c_char)
    -> swiboe::Result<*mut client::rpc::client::Context>
{
    let rpc_name_c_buf = unsafe {
        CStr::from_ptr(rpc_name)
    };
    let rpc_name = to_str_or_die(&rpc_name_c_buf);
    let args = if args.is_null() {
        serde_json::Value::Null
    } else {
        let args_c_str = unsafe {
            CStr::from_ptr(args)
        };
        to_json_or_die(&args_c_str)
    };

    let client_context = try!(context.call(&rpc_name, &args));
    Ok(unsafe {
        mem::transmute(Box::new(client_context))
    })
}

#[no_mangle]
pub extern "C" fn swiboe_server_context_call_rpc(
    context: *const client::rpc::server::Context,
    rpc_name: *const c_char,
    args: *const c_char,
    client_context: *mut *mut client::rpc::client::Context) -> CApiResult {
    let context: &mut client::rpc::server::Context = unsafe {
        mem::transmute(context)
    };

    unsafe {
        *client_context = try_capi!(call(context, rpc_name, args));
    }
    SUCCESS
}


#[no_mangle]
pub extern "C" fn swiboe_client_call_rpc(client: *const client::Client,
                                         rpc_name: *const c_char,
                                         args: *const c_char,
                                         client_context: *mut *mut client::rpc::client::Context) -> CApiResult {
    let client: &mut client::Client = unsafe {
        mem::transmute(client)
    };
    unsafe {
        *client_context = try_capi!(call(client, rpc_name, args));
    }
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_client_context_wait(context: *mut client::rpc::client::Context, rpc_result: *mut *const rpc::Result) -> CApiResult {
    let mut context: Box<client::rpc::client::Context> = unsafe {
        mem::transmute(context)
    };

    let result: rpc::Result = try_capi!(context.wait());
    unsafe {
        *rpc_result = mem::transmute(Box::new(result))
    }
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_client_context_recv(context: *mut client::rpc::client::Context, json_c_str: *mut *const c_char) -> CApiResult {
    // We expect the input parameter to be an unallocated string.
    assert_eq!(ptr::null(), unsafe { *json_c_str });

    let mut context: &mut client::rpc::client::Context = unsafe {
        mem::transmute(context)
    };

    let object = try_capi!(context.recv());
    match object {
        None => (),
        Some(json_value) => {
            let json_string = serde_json::to_string(&json_value).unwrap();
            let c_json_str = CString::new(json_string).unwrap().into_raw();
            unsafe {
                *json_c_str = c_json_str;
            }
        }
    }
    SUCCESS
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_result_is_ok(rpc_result: *const rpc::Result) -> bool {
    let rpc_result: &rpc::Result = unsafe {
        mem::transmute(rpc_result)
    };
    rpc_result.is_ok()
}

#[no_mangle]
pub extern "C" fn swiboe_rpc_result_unwrap(rpc_result: *const rpc::Result) -> *const c_char {
    // NOCOM(#sirver): deletes the object, needs to be documented.
    let rpc_result: Box<rpc::Result> = unsafe {
        mem::transmute(rpc_result)
    };
    let json_value = rpc_result.unwrap();
    let json_string = serde_json::to_string(&json_value).unwrap();
    CString::new(json_string).unwrap().into_raw()
}


pub type CCallback = extern fn(*mut client::rpc::server::Context, *const c_char);
struct CallbackRpc {
    priority: u16,
    callback: CCallback,
}

impl client::rpc::server::Rpc for CallbackRpc {
    fn priority(&self) -> u16 { self.priority }

    fn call(&self,
            context: client::rpc::server::Context,
            args: serde_json::Value) {
        let args_str = serde_json::to_string(&args).unwrap();
        let c_str = CString::new(args_str).expect("JSON contained zero byte");

        unsafe {
            let context_ptr: *mut client::rpc::server::Context =
                mem::transmute(Box::new(context));
            (self.callback)(context_ptr, c_str.as_ptr());
        };
    }
}

#[no_mangle]
pub extern "C" fn swiboe_new_rpc(client: *mut client::Client,
                                 rpc_name: *const c_char,
                                 priority: libc::uint16_t,
                                 callback: CCallback
                                 ) -> CApiResult {
    let client: &mut client::Client = unsafe {
        mem::transmute(client)
    };
    let rpc_name_cstr = unsafe {
        CStr::from_ptr(rpc_name)
    };

    let rpc = Box::new(CallbackRpc {
        priority: priority,
        callback: callback,
    });

    let rpc_name = to_str_or_die(rpc_name_cstr);
    try_capi!(client.new_rpc(rpc_name, rpc));
    SUCCESS
}
