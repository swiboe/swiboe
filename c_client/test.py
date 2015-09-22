#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

import argparse
import json
import os
import subprocess
import sys
import time
import unittest

from swiboe import SwiboeLibrary
import swiboe

class TestSwiboeClientLowLevel(unittest.TestCase):
    SHARED_LIBRARY = None
    TEST_SERVER = None

    def setUp(self):
        self.server_process = subprocess.Popen(
                [self.TEST_SERVER], stdout = subprocess.PIPE)
        self.test_dir = self.server_process.stdout.readline().strip()
        # NOCOM(#sirver): this should just be in the json file.
        self.socket_name = os.path.join(self.test_dir, "_socket")

        self.library = SwiboeLibrary(self.SHARED_LIBRARY)

    def tearDown(self):
        self.server_process.kill()

    def test_connect_and_disconnect(self):
        client = self.library.swiboe_connect(self.socket_name)
        self.library.swiboe_disconnect(client)

    def test_new_rpc(self):
        client = self.library.swiboe_connect(self.socket_name)

        def callback(server_context, args_string):
            return
        rpc_callback = swiboe.RPC(callback)
        self.library.swiboe_new_rpc(client, "list_rust_files", 100, rpc_callback)
        self.library.swiboe_disconnect(client)

    def test_call_successfull_rpc(self):
        serving_client = self.library.swiboe_connect(self.socket_name)
        golden_return = { "blub": "foo" }
        def callback(server_context, args_string):
            call_result = self.library.swiboe_rpc_ok(json.dumps(golden_return))
            self.library.swiboe_server_context_finish(server_context, call_result)
        rpc_callback = swiboe.RPC(callback)
        self.library.swiboe_new_rpc(serving_client, "test.test", 100, rpc_callback)

        client = self.library.swiboe_connect(self.socket_name)
        client_context = self.library.swiboe_client_call_rpc(
                client, "test.test", "null")
        call_result = self.library.swiboe_client_context_wait(client_context)
        self.assertTrue(self.library.swiboe_rpc_result_is_ok(call_result));

        json_blob = self.library.swiboe_rpc_result_unwrap(call_result)
        print "#sirver json_blob: %r" % (json_blob)
        self.assertEqual(golden_return, json.loads(json_blob))

        self.library.swiboe_disconnect(client)
        self.library.swiboe_disconnect(serving_client)




# NOCOM(#sirver): delete main function.
# def main():
    # client = library.swiboe_connect("/tmp/blub.socket")

    # def callback(server_context, args_string):
        # args = json.loads(args_string)
        # print("callback called %s" % args)
        # directory = args['directory']

        # client_context = library.swiboe_client_call_rpc(client, "list_files", json.dumps({
            # "directory": directory,
        # }))

        # # NOCOM(#sirver): look into returning errors from RPCs, not only successes.
        # while True:
            # json_blob = library.swiboe_client_context_recv(client_context)
            # if json_blob is None:
                # break
            # value = json.loads(json_blob)
            # value['files'] = [ v for v in value['files'] if
                    # v.endswith(".rs") or v.endswith(".toml") ]
            # value_str = json.dumps(value)
            # print "#sirver value_str: %r" % (value_str)
            # library.swiboe_server_context_update(server_context, value_str)

        # call_result = library.swiboe_client_context_wait(client_context)
        # library.swiboe_server_context_finish(server_context, call_result)


        # result = library.swiboe_rpc_ok("{}")
        # client_context = library.swiboe_server_context_call_rpc(server_context,
                # "test.test1", "{}")
        # call_result = library.swiboe_client_context_wait(client_context)
        # if library.swiboe_rpc_result_is_ok(call_result):
            # print "RPC call was okay."
        # json_blob = library.swiboe_rpc_result_unwrap(call_result)
        # print "#sirver json_blob: %r" % (json_blob)
        # library.swiboe_server_context_finish(server_context, result)
        # # server_context is no longer valid.

    # rpc_callback = swiboe.RPC(callback)

    # # TODO(sirver): The client should complain if the same RPC is registered twice.
    # library.swiboe_new_rpc(client, "list_rust_files", 100, rpc_callback)

    # while 1:
        # time.sleep(1)

    # library.swiboe_disconnect(client)

def flatten_test_suite(suite):
        flatten = unittest.TestSuite()
        for test in suite:
            if isinstance(test, unittest.TestSuite):
                flatten.addTests(flatten_test_suite(test))
            else:
                flatten.addTest(test)
        return flatten

def parse_args():
    p = argparse.ArgumentParser(description=
        "Test runner for the python interface. Also tests the full C ABI."
    )

    p.add_argument("--shared_library", type=str, default=None, help = "The shared library that will be loaded by ctypes for testing.")
    p.add_argument("--test_server", type=str, default="../target/debug/test_server", help=
            "The test server binary.")
    p.add_argument('-v', '--verbose', action='store_true', default=False,
                     help='print name of tests as they are executed')

    return p.parse_args()

def main():
    args = parse_args()

    all_test_suites = unittest.defaultTestLoader.discover(start_dir='.')
    suite = unittest.TestSuite()

    tests = set()
    for test in flatten_test_suite(all_test_suites):
        test.SHARED_LIBRARY = args.shared_library
        test.TEST_SERVER = args.test_server
        tests.add(test)
    suite.addTests(tests)
    # TODO(sirver): Add -v flag.
    successful = unittest.TextTestRunner(verbosity=0 if not args.verbose else 3,
                                          failfast=False).run(suite).wasSuccessful()
    return 0 if successful else 1


if __name__ == '__main__':
    sys.exit(main())
