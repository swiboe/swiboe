#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

from ctypes import c_void_p, c_char_p, c_uint16, c_int32, CFUNCTYPE, POINTER
import ctypes
import json
import os
import platform
import time
import sys

PtrClient = c_void_p
PtrClientContext = c_void_p
PtrRpcResult = c_void_p
PtrServerContext = c_void_p
Result = c_int32

RPC = CFUNCTYPE(None, PtrServerContext, c_char_p)

# Error codes
SUCCESS = 0
ERR_DISCONNECTED = 1
ERR_IO = 2
ERR_JSON_PARSING = 3
ERR_RPC_DONE = 4
ERR_INVALID_UTF8 = 5

def load_shared_library(shared_library):
    # TODO(sirver): This needs extending for Windows.
    if shared_library is None:
        shared_library = os.getenv('SWIBOE_LIBRARY')

    if shared_library is None:
        if platform.system() == 'Darwin':
            shared_library = 'libswiboe.dylib'
        else:
            shared_library = 'libswiboe.so'
    library = ctypes.cdll.LoadLibrary(shared_library)

    library.swiboe_connect.restype = Result
    library.swiboe_connect.argtypes = [c_char_p, POINTER(PtrClient)]

    library.swiboe_disconnect.restype = Result
    library.swiboe_disconnect.argtypes = [PtrClient]

    # TODO(sirver): The client should complain if the same RPC is registered
    # twice.
    library.swiboe_new_rpc.restype = Result
    library.swiboe_new_rpc.argtypes = [PtrClient, c_char_p, c_uint16, RPC]

    library.swiboe_rpc_ok.restype = PtrRpcResult
    library.swiboe_rpc_ok.argtypes = [c_char_p]

    library.swiboe_rpc_not_handled.restype = PtrRpcResult
    library.swiboe_rpc_not_handled.argtypes = []

    library.swiboe_rpc_error.restype = PtrRpcResult
    library.swiboe_rpc_error.argtypes = [c_char_p, c_char_p]

    library.swiboe_client_call_rpc.restype = PtrClientContext
    library.swiboe_client_call_rpc.argtypes = [PtrClient, c_char_p, c_char_p]

    library.swiboe_client_context_wait.restype = PtrRpcResult
    library.swiboe_client_context_wait.argtypes = [PtrClientContext]

    library.swiboe_server_context_finish.restype = None
    library.swiboe_server_context_finish.argtypes = [
        PtrServerContext, PtrRpcResult]

    library.swiboe_server_context_call_rpc.restype = PtrClientContext
    library.swiboe_server_context_call_rpc.argtypes = [
        PtrServerContext, c_char_p, c_char_p]

    library.swiboe_rpc_result_is_ok.restype = bool
    library.swiboe_rpc_result_is_ok.argtypes = [PtrRpcResult]

    library.swiboe_rpc_result_unwrap.restype = c_char_p
    library.swiboe_rpc_result_unwrap.argtypes = [PtrRpcResult]

    library.swiboe_client_context_recv.restype = c_char_p
    library.swiboe_client_context_recv.argtypes = [PtrClientContext]

    library.swiboe_server_context_update.restype = Result
    library.swiboe_server_context_update.argtypes = [PtrServerContext, c_char_p]

    return library


class SwiboeLibrary(object):
    LIBRARY = None
    LIBRARY_LOADING_ARGUMENT = None

    def __init__(self, shared_library=None):
        if SwiboeLibrary.LIBRARY is None:
            SwiboeLibrary.LIBRARY = load_shared_library(shared_library)
            SwiboeLibrary.LIBRARY_LOADING_ARGUMENT = shared_library
        elif SwiboeLibrary.LIBRARY_LOADING_ARGUMENT != shared_library:
            raise RuntimeError(
                'SwiboeLibrary() initialized with different arguments (was: %r, is: %r)' %
                (SwiboeLibrary.LIBRARY_LOADING_ARGUMENT, shared_library))

    def __getattribute__(self, name):
        # Using __getattr__ actually crashes here.
        return SwiboeLibrary.LIBRARY.__getattribute__(name)
