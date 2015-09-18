#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

from ctypes import c_void_p, c_char_p, c_uint16, CFUNCTYPE
import ctypes
import json
import os
import platform
import time

# TODO(sirver): This needs extending for Windows.
LIBRARY_NAME = os.getenv("SWIBOE_LIBRARY")
if LIBRARY_NAME is None:
    if platform.system() == "Darwin":
        LIBRARY_NAME = "libswiboe.dylib"
    else:
        LIBRARY_NAME = "libswiboe.so"
library = ctypes.cdll.LoadLibrary(LIBRARY_NAME)

PtrClient = c_void_p
library.swiboe_connect.restype = PtrClient
library.swiboe_connect.argtypes = [c_char_p]

library.swiboe_disconnect.restype = None
library.swiboe_disconnect.argtypes = [PtrClient]

PtrRpcResult = c_void_p
PtrServerContext = c_void_p
RPC = CFUNCTYPE(None, PtrServerContext, c_char_p)

library.swiboe_new_rpc.restype = None
library.swiboe_new_rpc.argtypes = [PtrClient, c_char_p, c_uint16, RPC]

library.swiboe_rpc_ok.restype = PtrRpcResult
library.swiboe_rpc_ok.argtypes = [c_char_p]

library.swiboe_rpc_not_handled.restype = PtrRpcResult
library.swiboe_rpc_not_handled.argtypes = []

library.swiboe_rpc_error.restype = PtrRpcResult
library.swiboe_rpc_error.argtypes = [c_char_p, c_char_p]

PtrClientContext = c_void_p
library.swiboe_client_call_rpc.restype = PtrClientContext
library.swiboe_client_call_rpc.argtypes = [PtrClient, c_char_p, c_char_p]

library.swiboe_client_context_wait.restype = PtrRpcResult
library.swiboe_client_context_wait.argtypes = [PtrClientContext]

library.swiboe_server_context_finish.restype = None
library.swiboe_server_context_finish.argtypes = [PtrServerContext, PtrRpcResult]

library.swiboe_server_context_call_rpc.restype = PtrClientContext
library.swiboe_server_context_call_rpc.argtypes = [PtrServerContext, c_char_p, c_char_p]

library.swiboe_rpc_result_is_ok.restype = bool
library.swiboe_rpc_result_is_ok.argtypes = [PtrRpcResult]

library.swiboe_rpc_result_unwrap.restype = c_char_p
library.swiboe_rpc_result_unwrap.argtypes = [PtrRpcResult]

library.swiboe_client_context_recv.restype = c_char_p
library.swiboe_client_context_recv.argtypes = [PtrClientContext]

library.swiboe_server_context_update.restype = None
library.swiboe_server_context_update.argtypes = [PtrServerContext, c_char_p]
