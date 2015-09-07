#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

from ctypes import c_void_p, c_char_p
import ctypes
import time

swiboe = ctypes.cdll.LoadLibrary("target/debug/libswiboe.dylib")

swiboe.create_client.restype = c_void_p
swiboe.create_client.argtypes = [c_char_p]

swiboe.disconnect.restype = None
swiboe.disconnect.argtypes = [c_void_p]

client = swiboe.create_client("/tmp/blub.socket")

time.sleep(5)

swiboe.disconnect(client)
