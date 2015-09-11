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
RPC = CFUNCTYPE(PtrRpcResult, c_char_p)

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

swiboe.swiboe_rpc_context_wait.restype = None
swiboe.swiboe_rpc_context_wait.argtypes = [PtrClientContext]

serving_client = swiboe.swiboe_connect("/tmp/blub.socket")
# TODO(sirver): handle streaming rpcs.
# TODO(sirver): Call other rpcs.

def callback(args):
    print("callback called %s" % args)
    return swiboe.swiboe_rpc_ok("{}")

rpc_callback = RPC(callback)

# TODO(sirver): The serving_client should complain if the same RPC is registered twice.
swiboe.swiboe_new_rpc(serving_client, "test.test", 100, rpc_callback)

clients = [ swiboe.swiboe_connect("/tmp/blub.socket") for i in range(50) ]
contexts = []
num = 0
for c in clients:
    for i in range(100):
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

swiboe.swiboe_disconnect(serving_client)
