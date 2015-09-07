#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

import ctypes
sw = ctypes.cdll.LoadLibrary("target/debug/libswiboe.dylib")

sw.hello(u"Löwe 老虎 Léopard".encode('utf-8'))
sw.hello(u"ASCII żółć 🇨🇭 한".encode('utf-8'))

def callback(arg):
    return sw.create("Hello again: %i" % arg)

sw.create.restype = ctypes.c_void_p

CALLBACK = ctypes.CFUNCTYPE(ctypes.c_void_p, ctypes.c_int32)
sw.hello1(CALLBACK(callback))
