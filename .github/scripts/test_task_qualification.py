"""Native qualification must observe closure before reusing a thread UUID."""
import importlib.util
import hashlib
import json
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import unittest


spec = importlib.util.spec_from_file_location(
    "qualify_tasks", Path(__file__).resolve().parents[2] / "scripts/qualify_tasks.py"
)
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)
from rald5_publication import EXPECTED_FILES, ValidationError


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


class AuthorityTests(unittest.TestCase):
    def test_fixture_trust_comes_from_explicit_ed25519_public_authority(self):
        with tempfile.TemporaryDirectory(prefix="task-authority-") as temporary:
            root = Path(temporary)
            private, public = root / "private.pem", root / "public.pem"
            subprocess.run(["openssl", "genpkey", "-algorithm", "Ed25519", "-out", str(private)], check=True, capture_output=True)
            subprocess.run(["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)], check=True, capture_output=True)
            expected = subprocess.check_output(["openssl", "pkey", "-pubin", "-in", str(public), "-outform", "DER"])[-32:].hex()
            state = dict(row.split("=", 1) for row in qualification.fixture_activation_state("owned", public).splitlines())
            self.assertEqual(state["current"], "owned")
            self.assertEqual(state["update_key"], expected)
            self.assertEqual(state["current_key"], expected)
            self.assertEqual(state["previous_present"], "0")

    def test_fixture_rejects_other_public_key_algorithm(self):
        with tempfile.TemporaryDirectory(prefix="task-authority-") as temporary:
            root = Path(temporary)
            private, public = root / "private.pem", root / "public.pem"
            subprocess.run(["openssl", "genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-out", str(private)], check=True, capture_output=True)
            subprocess.run(["openssl", "pkey", "-in", str(private), "-pubout", "-out", str(public)], check=True, capture_output=True)
            with self.assertRaisesRegex(ValidationError, "Ed25519"):
                qualification.fixture_activation_state("owned", public)


class MaterializationTests(unittest.TestCase):
    def test_only_signed_installed_inventory_is_copied_without_payload_rewrite(self):
        with tempfile.TemporaryDirectory(prefix="task-inventory-") as temporary:
            root = Path(temporary); source, native = root / "published", root / "installed"
            source.mkdir()
            headers = [("generation_id", "local-hosted-0-160-0-owned"), ("release_sequence", "38"), ("channel", "stable"), ("expected_platform", "android"), ("expected_architecture", "aarch64"), ("core_api_identity", "core-api-v1"), ("persistent_schema_identity", "schema-v1"), ("release_public_key", "11" * 32), ("file_count", str(len(EXPECTED_FILES)))]
            records = []
            for relative, mode in EXPECTED_FILES.items():
                path = source / relative; path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(relative.encode()); path.chmod(int(mode, 8))
                records.append("file\t" + relative + "\t" + hashlib.sha256(path.read_bytes()).hexdigest() + "\t" + mode + "\n")
            manifest = source / "release.manifest"
            manifest.write_text("codex-release-v4\n" + "".join(k + "\t" + v + "\n" for k, v in headers) + "".join(records))
            (source / "release.sig").write_bytes(b"signed control")
            (source / "compat").mkdir(); (source / "compat/download-size-v1").write_bytes(b"published sidecar")
            (source / "update-index-v1").write_bytes(b"published channel control")
            qualification.materialize_signed_generation(source, native)
            expected = set(EXPECTED_FILES) | {"release.manifest", "release.sig"}
            self.assertEqual({str(p.relative_to(native)) for p in native.rglob("*") if p.is_file()}, expected)
            for relative in expected:
                self.assertEqual((native / relative).read_bytes(), (source / relative).read_bytes())
                self.assertEqual((native / relative).stat().st_mode, (source / relative).stat().st_mode)
            self.assertFalse((native / "compat").exists())
            manifest.write_text(manifest.read_text().replace("file_count\t7", "file_count\t8") + "file\tcompat/download-size-v1\t" + "0" * 64 + "\t0644\n")
            with self.assertRaises(ValidationError):
                qualification.materialize_signed_generation(source, root / "rejected")
            self.assertFalse((root / "rejected").exists())


if __name__ == "__main__":
    unittest.main()
