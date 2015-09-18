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
swiboe.swiboe_rpc_context_wait.restype = None
swiboe.swiboe_rpc_context_wait.argtypes = [PtrClientContext]

swiboe.swiboe_server_context_finish.restype = None
swiboe.swiboe_server_context_finish.argtypes = [PtrServerContext, PtrRpcResult]

swiboe.swiboe_server_context_call_rpc.restype = PtrClientContext
swiboe.swiboe_server_context_call_rpc.argtypes = [PtrServerContext, c_char_p, c_char_p]

serving_client = swiboe.swiboe_connect("/tmp/blub.socket")
serving_client1 = swiboe.swiboe_connect("/tmp/blub.socket")
# TODO(sirver): handle streaming rpcs.

def callback1(rpc_context, args):
    print "#sirver args: %r" % (args)
    result = swiboe.swiboe_rpc_ok("""{ "foo": "blah" }""")
    swiboe.swiboe_server_context_finish(rpc_context, result)

def callback(rpc_context, args):
    print("callback called %s" % args)
    result = swiboe.swiboe_rpc_ok("{}")
    # client_context = swiboe.swiboe_server_context_call_rpc(rpc_context,
            # "test.test1", "{}")
    # TODO(sirver): look into getting results back.
    # swiboe.swiboe_rpc_context_wait(client_context)
    swiboe.swiboe_server_context_finish(rpc_context, result)
    # rpc_context is no longer valid.

rpc_callback = RPC(callback)
rpc_callback1 = RPC(callback1)

# TODO(sirver): The serving_client should complain if the same RPC is registered twice.
swiboe.swiboe_new_rpc(serving_client, "test.test", 100, rpc_callback)
swiboe.swiboe_new_rpc(serving_client1, "test.test1", 100, rpc_callback1)

clients = [ swiboe.swiboe_connect("/tmp/blub.socket") for i in range(5) ]
contexts = []
num = 0
for c in clients:
    for i in range(10):
        contexts.append(
                swiboe.swiboe_client_call_rpc(c, "test.test", json.dumps({
                    "num": num })))
        num += 1

for context in contexts:
    swiboe.swiboe_rpc_context_wait(context)

for c in clients:
    swiboe.swiboe_disconnect(c)

# while 1:
    # time.sleep(1)

swiboe.swiboe_disconnect(serving_client1)
swiboe.swiboe_disconnect(serving_client)
