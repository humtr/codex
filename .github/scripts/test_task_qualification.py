"""Native qualification must observe closure before reusing a thread UUID."""
import importlib.util
import json
from pathlib import Path
import socket
import struct
import unittest


spec = importlib.util.spec_from_file_location(
    "qualify_tasks", Path(__file__).resolve().parents[2] / "scripts/qualify_tasks.py"
)
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)


class ClosureTests(unittest.TestCase):
    def setUp(self):
        client, self.server = socket.socketpair()
        self.rpc = object.__new__(qualification.Ws)
        self.rpc.socket = client
        self.rpc.socket.settimeout(1)
        self.rpc.pending = b""
        self.rpc.next = 0
        self.rpc.closed = set()
        self.addCleanup(self.rpc.close)
        self.addCleanup(self.server.close)

    def send(self, value):
        data = json.dumps(value).encode()
        header = bytes([0x81, len(data)]) if len(data) < 126 else bytes([0x81, 126]) + struct.pack("!H", len(data))
        self.server.sendall(header + data)

    def closed(self, thread):
        self.send({"method": "thread/closed", "params": {"threadId": thread}})

    def test_closure_observed_during_rpc_is_retained(self):
        self.closed("owned")
        self.send({"id": 1, "result": {"ok": True}})
        self.assertEqual(self.rpc.call("thread/loaded/list", {}), {"ok": True})
        self.rpc.wait_closed("owned", timeout=0)

    def test_other_thread_or_idle_notification_is_not_closure(self):
        self.closed("other")
        self.send({"method": "thread/status/changed", "params": {"threadId": "owned", "status": "idle"}})
        self.closed("owned")
        self.rpc.wait_closed("owned")
        self.assertEqual(self.rpc.closed, {"other", "owned"})
        self.assertEqual(self.rpc.socket.gettimeout(), 1)

    def test_missing_closure_fails_bounded_and_restores_timeout(self):
        with self.assertRaises(TimeoutError):
            self.rpc.wait_closed("owned", timeout=0.02)
        self.assertEqual(self.rpc.socket.gettimeout(), 1)

    def test_resume_requires_new_closure_for_same_uuid(self):
        self.rpc.closed.add("owned")
        self.send({"id": 1, "result": {}})
        self.rpc.call("thread/resume", {"threadId": "owned"})
        self.assertNotIn("owned", self.rpc.closed)
        self.closed("owned")
        self.rpc.wait_closed("owned")


if __name__ == "__main__":
    unittest.main()
