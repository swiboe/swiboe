#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

from ctypes import c_void_p, c_char_p, c_uint16, CFUNCTYPE
import ctypes
import json
import time

swiboe = ctypes.cdll.LoadLibrary("target/debug/libswiboe.dylib")

PtrClient = c_void_p
swiboe.swiboe_connect.restype = PtrClient
swiboe.swiboe_connect.argtypes = [c_char_p]

swiboe.swiboe_disconnect.restype = None
swiboe.swiboe_disconnect.argtypes = [PtrClient]

PtrRpcResult = c_void_p
PtrServerContext = c_void_p
RPC = CFUNCTYPE(None, PtrServerContext, c_char_p)

swiboe.swiboe_new_rpc.restype = None
swiboe.swiboe_new_rpc.argtypes = [PtrClient, c_char_p, c_uint16, RPC]

swiboe.swiboe_rpc_ok.restype = PtrRpcResult
swiboe.swiboe_rpc_ok.argtypes = [c_char_p]

swiboe.swiboe_rpc_not_handled.restype = PtrRpcResult
swiboe.swiboe_rpc_not_handled.argtypes = []

swiboe.swiboe_rpc_error.restype = PtrRpcResult
swiboe.swiboe_rpc_error.argtypes = [c_char_p, c_char_p]

PtrClientContext = c_void_p
swiboe.swiboe_client_call_rpc.restype = PtrClientContext
swiboe.swiboe_client_call_rpc.argtypes = [PtrClient, c_char_p, c_char_p]

# NOCOM(#sirver): rename rpc_context to either client_context or server_context
swiboe.swiboe_rpc_context_wait.restype = PtrRpcResult
swiboe.swiboe_rpc_context_wait.argtypes = [PtrClientContext]

swiboe.swiboe_server_context_finish.restype = None
swiboe.swiboe_server_context_finish.argtypes = [PtrServerContext, PtrRpcResult]

swiboe.swiboe_server_context_call_rpc.restype = PtrClientContext
swiboe.swiboe_server_context_call_rpc.argtypes = [PtrServerContext, c_char_p, c_char_p]

swiboe.swiboe_rpc_result_is_ok.restype = bool
swiboe.swiboe_rpc_result_is_ok.argtypes = [PtrRpcResult]

swiboe.swiboe_rpc_result_unwrap.restype = c_char_p
swiboe.swiboe_rpc_result_unwrap.argtypes = [PtrRpcResult]

swiboe.swiboe_rpc_context_recv.restype = c_char_p
swiboe.swiboe_rpc_context_recv.argtypes = [PtrClientContext]

swiboe.swiboe_server_context_update.restype = None
swiboe.swiboe_server_context_update.argtypes = [PtrServerContext, c_char_p]

client = swiboe.swiboe_connect("/tmp/blub.socket")
# TODO(sirver): handle streaming rpcs.

def callback(server_context, args_string):
    args = json.loads(args_string)
    print("callback called %s" % args)
    directory = args['directory']

    client_context = swiboe.swiboe_client_call_rpc(client, "list_files", json.dumps({
        "directory": directory,
    }))

    # NOCOM(#sirver): look into returning errors from RPCs, not only successes.
    while True:
        json_blob = swiboe.swiboe_rpc_context_recv(client_context)
        if json_blob is None:
            break
        value = json.loads(json_blob)
        value['files'] = [ v for v in value['files'] if
                v.endswith(".rs") or v.endswith(".toml") ]
        value_str = json.dumps(value)
        print "#sirver value_str: %r" % (value_str)
        swiboe.swiboe_server_context_update(server_context, value_str)

    call_result = swiboe.swiboe_rpc_context_wait(client_context)
    swiboe.swiboe_server_context_finish(server_context, call_result)


    # result = swiboe.swiboe_rpc_ok("{}")
    # client_context = swiboe.swiboe_server_context_call_rpc(rpc_context,
            # "test.test1", "{}")
    # # TODO(sirver): look into getting results back.
    # call_result = swiboe.swiboe_rpc_context_wait(client_context)
    # if swiboe.swiboe_rpc_result_is_ok(call_result):
        # print "RPC call was okay."
    # json_blob = swiboe.swiboe_rpc_result_unwrap(call_result)
    # print "#sirver json_blob: %r" % (json_blob)
    # swiboe.swiboe_server_context_finish(rpc_context, result)
    # # rpc_context is no longer valid.

rpc_callback = RPC(callback)

# TODO(sirver): The client should complain if the same RPC is registered twice.
swiboe.swiboe_new_rpc(client, "list_rust_files", 100, rpc_callback)

while 1:
    time.sleep(1)

swiboe.swiboe_disconnect(client)
