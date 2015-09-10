#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

import ctypes
sw = ctypes.cdll.LoadLibrary("target/debug/libswiboe.dylib")

sw.hello(u"Löwe 老虎 Léopard".encode('utf-8'))
sw.hello(u"ASCII żółć 🇨🇭 한".encode('utf-8'))
