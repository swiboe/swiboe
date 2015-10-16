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

/// Local results.
#[allow(non_camel_case_types)]
#[repr(i32)]
pub enum CApiResult {
    SUCCESS = 0,
    ERR_DISCONNECTED = 1,
    ERR_IO = 2,
    ERR_JSON_PARSING = 3,
    ERR_RPC_DONE = 4,
    ERR_INVALID_UTF8 = 5,
}

// Like try!, but instead of Err() returns a CApiResult that represents the error.
macro_rules! try_capi {
    ($expr:expr) => (match $expr {
        ::std::result::Result::Ok(val) => val,
        ::std::result::Result::Err(swiboe_error) => {
            return match swiboe_error {
                swiboe::Error::Disconnected => CApiResult::ERR_DISCONNECTED,
                swiboe::Error::Io(_) => CApiResult::ERR_IO,
                swiboe::Error::JsonParsing(_) => CApiResult::ERR_JSON_PARSING,
                swiboe::Error::RpcDone => CApiResult::ERR_RPC_DONE,
                swiboe::Error::InvalidUtf8 => CApiResult::ERR_INVALID_UTF8,
            }
        }
    })
}

/// RPC errors.
#[allow(non_camel_case_types)]
#[repr(i32)]
pub enum CApiRpcErrorKind {
    RPC_ERR_UNKNOWN = 1,
    RPC_ERR_IO = 2,
    RPC_ERR_INVALID_ARGS = 3,
}


// Converts 'kind' to a matching rpc::ErrorKind enum.
fn to_rpc_error_kind(kind: CApiRpcErrorKind) -> rpc::ErrorKind {
    match kind {
        CApiRpcErrorKind::RPC_ERR_UNKNOWN => rpc::ErrorKind::UnknownRpc,
        CApiRpcErrorKind::RPC_ERR_IO => rpc::ErrorKind::Io,
        CApiRpcErrorKind::RPC_ERR_INVALID_ARGS => rpc::ErrorKind::InvalidArgs,
    }
}

// Converts a buffer we got passed from the API user or dies if the input is invalid.
fn to_str_or_die(c_str: &CStr) -> &str {
    str::from_utf8(c_str.to_bytes())
        .expect("argument was not a valid UTF-8 encoded string.")
}

// Converts a buffer we got passed from the API user or dies if the input is invalid.
fn to_json_or_die(c_str: &CStr) -> serde_json::Value {
    let json_str = to_str_or_die(c_str);
    let json_value: serde_json::Value = serde_json::from_str(json_str)
        .expect("argument was not valid JSON.");
    json_value
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

/// Connects to the Unix socket listening at 'socket_name' which must be a Swiboe server. Fills in
/// 'client' which must be deleted through 'swiboe_disconnect' once no longer used.
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
    CApiResult::SUCCESS
}

/// Disconnects 'client' from the server and deletes it.
#[no_mangle]
pub extern "C" fn swiboe_disconnect(client: *mut client::Client) -> CApiResult {
    unsafe {
        let _: Box<client::Client> = mem::transmute(client);
    }
    CApiResult::SUCCESS
}

/// Ends the RPC call from the server side by calling 'context.finish' with the given 'rpc_result'.
/// Will delete 'context' and 'rpc_result'.
#[no_mangle]
pub extern "C" fn swiboe_server_context_finish(context: *mut client::rpc::server::Context, rpc_result: *const rpc::Result) -> CApiResult {
    let mut context: Box<client::rpc::server::Context> = unsafe {
         mem::transmute(context)
    };
    let result: Box<rpc::Result> = unsafe {
         mem::transmute(rpc_result)
    };
    try_capi!(context.finish(*result));
    CApiResult::SUCCESS
}

/// Sends a partial reply for the current RPC by calling 'context.update'. Does not take ownership.
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
    CApiResult::SUCCESS
}

/// Creates a new rpc::Result::Ok with the given 'json_str'.
#[no_mangle]
pub extern "C" fn swiboe_rpc_ok(json_str: *const c_char) -> *const rpc::Result {
    let json_c_str = unsafe {
        CStr::from_ptr(json_str)
    };
    let json_value = to_json_or_die(&json_c_str);
    unsafe {
        mem::transmute(Box::new(rpc::Result::Ok(json_value)))
    }
}

/// Creates a new rpc::Result::Err with the given 'error_kind' and the given 'json_details' which
/// can be NULL.
#[no_mangle]
pub extern "C" fn swiboe_rpc_error(error_kind: CApiRpcErrorKind, json_details: *const c_char) -> *const rpc::Result {
    let details = if json_details.is_null() {
        None
    } else {
        let c_str_details = unsafe {
            CStr::from_ptr(json_details)
        };
        Some(to_json_or_die(&c_str_details))
    };

    let err = rpc::Error {
        kind: to_rpc_error_kind(error_kind),
        details: details,
    };

    unsafe {
        mem::transmute(Box::new(rpc::Result::Err(err)))
    }
}

/// Creates a new rpc::Result::NotHandled.
#[no_mangle]
pub extern "C" fn swiboe_rpc_not_handled() -> *const rpc::Result {
    unsafe {
        mem::transmute(Box::new(rpc::Result::NotHandled))
    }
}

// TODO(sirver): Wrap cancel for long running RPCs.

/// Calls another RPC from an RPC handler. Fills in 'client_context' with a new object that has to
/// be deleted through 'wait'.
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
    CApiResult::SUCCESS
}


/// Calls all layered RPCs matching 'rpc_selector' with 'args'. Fills in 'client_context' with a
/// new object that has to be deleted through 'wait'.
#[no_mangle]
pub extern "C" fn swiboe_client_call_rpc(client: *const client::Client,
                                         rpc_selector: *const c_char,
                                         args: *const c_char,
                                         client_context: *mut *mut client::rpc::client::Context) -> CApiResult {
    let client: &mut client::Client = unsafe {
        mem::transmute(client)
    };
    unsafe {
        *client_context = try_capi!(call(client, rpc_selector, args));
    }
    CApiResult::SUCCESS
}


/// Waits for the RPC to finish and deletes the 'context'. Fills in 'rpc_result' with the final
/// result which must be deleted using any of the unwrap() methods.
#[no_mangle]
pub extern "C" fn swiboe_client_context_wait(context: *mut client::rpc::client::Context, rpc_result: *mut *const rpc::Result) -> CApiResult {
    let mut context: Box<client::rpc::client::Context> = unsafe {
        mem::transmute(context)
    };

    let result: rpc::Result = try_capi!(context.wait());
    unsafe {
        *rpc_result = mem::transmute(Box::new(result))
    }
    CApiResult::SUCCESS
}

/// Blocks till a partial result is received for the RPC represented by 'context'. Fills in
/// 'json_c_str' with the new result. This has to be free() by the caller.
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
            // TODO(sirver): maybe this leaks memory. Does the c-land free() this correctly?
            let json_string = serde_json::to_string(&json_value).unwrap();
            let c_json_str = CString::new(json_string).unwrap().into_raw();
            unsafe {
                *json_c_str = c_json_str;
            }
        }
    }
    CApiResult::SUCCESS
}

// NOCOM(#sirver): we continue here tomorrow.
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

#[no_mangle]
pub extern "C" fn swiboe_rpc_result_unwrap_err(rpc_result: *const rpc::Result, details: *mut *const c_char) -> CApiRpcErrorKind {
    // NOCOM(#sirver): deletes the object, needs to be documented.
    assert_eq!(ptr::null(), unsafe { *details });

    let rpc_result: Box<rpc::Result> = unsafe {
        mem::transmute(rpc_result)
    };
    let err = rpc_result.unwrap_err();

    if let Some(err_details) = err.details {
        let json_string = serde_json::to_string(&err_details).unwrap();
        let details_c_str = CString::new(json_string).unwrap().into_raw();
        unsafe {
            *details = details_c_str;
        }
    }

    match err.kind {
        rpc::ErrorKind::UnknownRpc => CApiRpcErrorKind::RPC_ERR_UNKNOWN,
        rpc::ErrorKind::Io => CApiRpcErrorKind::RPC_ERR_IO,
        rpc::ErrorKind::InvalidArgs => CApiRpcErrorKind::RPC_ERR_INVALID_ARGS,
    }
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
    CApiResult::SUCCESS
}
