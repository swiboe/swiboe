import os
import socket
import struct
import sys
import time

class SupremeClient(object):

    """Docstring for SupremeClient. """

    def __init__(self, socket_name):
        """TODO: to be defined1. """
        self._socket_name = socket_name
        self._socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self._socket.connect(socket_name)


    def write_message(self, msg):
        data = struct.pack("<L", len(msg)) + msg
        self._socket.sendall(data)

    def call(self, function, args):
        d = {}
        d['function'] = function
        d['type'] = 'call'
        d['args'] = args
        client.write_message(json.dumps(d))

if __name__ == '__main__':
    import json
    client = SupremeClient("/tmp/s.socket")
    client.call("core.register_function", {
        "name": "python.test_client.hello_world"
    })

    # client.write_message(json.dumps({ "type": "call", "function": "core.exit" }))
    time.sleep(3)
