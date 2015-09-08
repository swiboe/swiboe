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

RPC = CFUNCTYPE(None)

swiboe.swiboe_new_rpc.restype = None
swiboe.swiboe_new_rpc.argtypes = [PtrClient, c_char_p, c_uint16, RPC]

client = swiboe.swiboe_connect("/tmp/blub.socket")

def callback():
    print("Called back!")

swiboe.swiboe_new_rpc(client, "test.test", 100, RPC(callback))

while 1:
    time.sleep(1)

swiboe.swiboe_disconnect(client)
