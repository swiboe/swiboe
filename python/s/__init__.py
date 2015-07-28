import os
import socket
import struct
import sys
import time
import uuid

def recv_all(socket, num_bytes):
    data = ""
    while num_bytes:
        new_data = socket.recv(num_bytes)
        num_bytes -= len(new_data)
        data += new_data
    return data

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

    def read_message(self):
        blob = recv_all(self._socket, 4)
        length, = struct.unpack("<L", blob)
        return recv_all(self._socket, length)

    def call(self, function, args):
        d = {}
        d['function'] = function
        d['type'] = 'call'
        d['context'] = str(uuid.uuid4())
        d['args'] = args
        client.write_message(json.dumps(d))

if __name__ == '__main__':
    import json
    import time

    client = SupremeClient("/tmp/s.socket")
    # client2 = SupremeClient("/tmp/s.socket")
    client.call("core.register_function", {
        "name": "python.test_client.hello_world"
    })

    # start = time.time()
    # NUM_RUNS = 1000
    # for i in range(NUM_RUNS):
        # client.call("core.broadcast", {
            # "blub": "blah"
        # })
        # msg = client.read_message()
        # # msg1 = client2.read_message()
        # assert msg == msg1
    # duration_in_seconds = time.time() - start
    # print "#sirver %fms per roundtrip." % (duration_in_seconds * 1000 / NUM_RUNS)

    # client.write_message(json.dumps({ "type": "call", "function": "core.exit" }))

    client.call("python.test_client.hello_world", {})
    msg = client.read_message()
    print "#sirver msg: %r" % (msg)
