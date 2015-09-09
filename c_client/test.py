#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

from ctypes import c_void_p, c_char_p, c_uint16, CFUNCTYPE
import ctypes
import time

swiboe = ctypes.cdll.LoadLibrary("target/debug/libswiboe.dylib")

PtrClient = c_void_p
swiboe.swiboe_connect.restype = PtrClient
swiboe.swiboe_connect.argtypes = [c_char_p]

swiboe.swiboe_disconnect.restype = None
swiboe.swiboe_disconnect.argtypes = [PtrClient]

RPC = CFUNCTYPE(c_uint16, c_char_p)

swiboe.swiboe_new_rpc.restype = None
swiboe.swiboe_new_rpc.argtypes = [PtrClient, c_char_p, c_uint16, RPC]

client = swiboe.swiboe_connect("/tmp/blub.socket")
client1 = swiboe.swiboe_connect("/tmp/blub.socket")

RPC_OK = 0
RPC_ERR = 1
RPC_NOT_HANDLED = 2

def callback(args):
    print("callback called!")
    return RPC_OK

def callback1(args):
    print("callback1 called!")
    return RPC_NOT_HANDLED

rpc_callback = RPC(callback)
rpc_callback1 = RPC(callback1)

# TODO(sirver): The client should complain if the same RPC is registered twice.
swiboe.swiboe_new_rpc(client, "test.test", 100, rpc_callback)
swiboe.swiboe_new_rpc(client1, "test.test", 50, rpc_callback1)

while 1:
    time.sleep(1)

swiboe.swiboe_disconnect(client)
