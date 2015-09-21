#!/usr/bin/env python
# -*- coding: utf-8 -*-
# Copyright (c) The Swiboe development team. All rights reserved.
# Licensed under the Apache License, Version 2.0. See LICENSE.txt
# in the project root for license information.

import json
import os
import subprocess
import time
import unittest

from swiboe import library
import swiboe

# NOCOM(#sirver): make SWIBOE_LIBRARY passable on the commandline for the test runner.

# NOCOM(#sirver): should not be hardcoded.
TEST_SERVER="../target/debug/test_server"

class TestSwiboeClientLowLevel(unittest.TestCase):

    def setUp(self):
        self.server_process = subprocess.Popen(
                [TEST_SERVER], stdout = subprocess.PIPE)
        self.test_dir = self.server_process.stdout.readline().strip()
        # NOCOM(#sirver): this should just be in the json file.
        self.socket_name = os.path.join(self.test_dir, "_socket")

    def tearDown(self):
        self.server_process.kill()

    # NOCOM(#sirver): all of these tests should bring up the swiboe server
    # themselves, so that we can run tests in isolation and parallel.
    def test_connect_and_disconnect(self):
        client = library.swiboe_connect(self.socket_name)
        library.swiboe_disconnect(client)

    def test_new_rpc(self):
        client = library.swiboe_connect(self.socket_name)

        def callback(server_context, args_string):
            return
        rpc_callback = swiboe.RPC(callback)
        library.swiboe_new_rpc(client, "list_rust_files", 100, rpc_callback)
        library.swiboe_disconnect(client)

    def test_call_successfull_rpc(self):
        serving_client = library.swiboe_connect(self.socket_name)
        golden_return = { "blub": "foo" }
        def callback(server_context, args_string):
            call_result = library.swiboe_rpc_ok(json.dumps(golden_return))
            library.swiboe_server_context_finish(server_context, call_result)
        rpc_callback = swiboe.RPC(callback)
        library.swiboe_new_rpc(serving_client, "test.test", 100, rpc_callback)

        client = library.swiboe_connect(self.socket_name)
        client_context = library.swiboe_client_call_rpc(
                client, "test.test", "null")
        call_result = library.swiboe_client_context_wait(client_context)
        self.assertTrue(library.swiboe_rpc_result_is_ok(call_result));

        json_blob = library.swiboe_rpc_result_unwrap(call_result)
        print "#sirver json_blob: %r" % (json_blob)
        self.assertEqual(golden_return, json.loads(json_blob))

        library.swiboe_disconnect(client)
        library.swiboe_disconnect(serving_client)




# NOCOM(#sirver): delete main function.
def main():
    client = library.swiboe_connect("/tmp/blub.socket")

    def callback(server_context, args_string):
        args = json.loads(args_string)
        print("callback called %s" % args)
        directory = args['directory']

        client_context = library.swiboe_client_call_rpc(client, "list_files", json.dumps({
            "directory": directory,
        }))

        # NOCOM(#sirver): look into returning errors from RPCs, not only successes.
        while True:
            json_blob = library.swiboe_client_context_recv(client_context)
            if json_blob is None:
                break
            value = json.loads(json_blob)
            value['files'] = [ v for v in value['files'] if
                    v.endswith(".rs") or v.endswith(".toml") ]
            value_str = json.dumps(value)
            print "#sirver value_str: %r" % (value_str)
            library.swiboe_server_context_update(server_context, value_str)

        call_result = library.swiboe_client_context_wait(client_context)
        library.swiboe_server_context_finish(server_context, call_result)


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

    rpc_callback = swiboe.RPC(callback)

    # TODO(sirver): The client should complain if the same RPC is registered twice.
    library.swiboe_new_rpc(client, "list_rust_files", 100, rpc_callback)

    while 1:
        time.sleep(1)

    library.swiboe_disconnect(client)

if __name__ == '__main__':
    unittest.main()
