#!/usr/bin/env python3
"""One owned D3 MCP stdio fault: preserve a returned result, then close the link.

The Provider/native executor is unchanged. Never retry a request. The effect file
and exact tools/call response must both exist before the cut can fire.
"""
import argparse
import hashlib
import json
import os
import shlex
from pathlib import Path
import subprocess
import sys
import threading
import time


def run(root, command):
    requests = {}
    process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=sys.stderr, start_new_session=True)

    def forward():
        try:
            for line in sys.stdin.buffer:
                value = json.loads(line)
                if "id" in value:
                    requests[str(value["id"])] = value
                process.stdin.write(line)
                process.stdin.flush()
        except (BrokenPipeError, ValueError, OSError):
            pass
        finally:
            try:
                process.stdin.close()
            except OSError:
                pass

    threading.Thread(target=forward, daemon=True).start()
    try:
        for line in process.stdout:
            value = json.loads(line)
            request = requests.get(str(value.get("id")), {})
            name = request.get("params", {}).get("name")
            effects = root / "effects/fault.effects"
            eligible = (root / "fault-armed").exists() and effects.exists()
            eligible = eligible and effects.read_text() == "fault\n"
            eligible = eligible and request.get("method") == "tools/call"
            eligible = eligible and name in ("native_tool", "codex_execute")
            arguments = request.get("params", {}).get("arguments", {})
            try:
                native = json.loads(arguments.get("input", "{}"))
                exact_probe = shlex.split(native.get("cmd", "")) == ["python3", "-B", "probe.py", "--root", "../effects", "--label", "fault"]
            except (ValueError, TypeError):
                exact_probe = False
            eligible = eligible and arguments.get("name", "").endswith("exec_command") and exact_probe
            eligible = eligible and b"D3_EFFECT_RETURNED_FAULT" in line and "result" in value
            if eligible:
                try:
                    fd = os.open(root / "fault-fired.json", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
                except FileExistsError:
                    pass
                else:
                    with (root / "withheld-mcp-result-private.json").open("xb") as original:
                        original.write(line)
                        original.flush()
                        os.fsync(original.fileno())
                    with os.fdopen(fd, "w") as output:
                        json.dump({"UTC_ns": time.time_ns(), "monotonic_ns": time.monotonic_ns(),
                                   "rpc_id": value["id"], "control": name,
                                   "request_sha256": hashlib.sha256(json.dumps(request, sort_keys=True).encode()).hexdigest(),
                                   "result_sha256": hashlib.sha256(line).hexdigest(),
                                   "effect_count": 1, "forwarded_to_Chat": False}, output)
                        output.flush()
                        os.fsync(output.fileno())
                    return 42
            sys.stdout.buffer.write(line)
            sys.stdout.buffer.flush()
        return process.wait(timeout=5)
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == "__main__":
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    assert args.root.is_dir() and command
    raise SystemExit(run(args.root, command))
