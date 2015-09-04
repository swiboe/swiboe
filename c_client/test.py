#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

import ctypes
sw = ctypes.cdll.LoadLibrary("target/debug/libswiboe.dylib")

sw.hello(u"Löwe 老虎 Léopard".encode('utf-8'))
sw.hello(u"ASCII żółć 🇨🇭 한".encode('utf-8'))

def callback(arg, str_pointer):
    str_pointer = "Hello again: %i" % arg


CALLBACK = ctypes.CFUNCTYPE(None, ctypes.c_int32, ctypes.POINTER(ctypes.c_char_p))
sw.hello1(CALLBACK(callback))
