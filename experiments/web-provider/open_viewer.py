"""Open the private temporary viewer through Termux's existing AM socket."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
from urllib.parse import urlencode


def viewer_url(state):
    assert type(state["port"]) is int and 0 < state["port"] < 65536
    assert isinstance(state["capability"], str) and re.fullmatch(r"[0-9a-f]{64}", state["capability"])
    query = urlencode({"autoconnect": "1", "resize": "remote", "path": "/" + state["capability"] + "/websock"})
    return f'http://127.0.0.1:{state["port"]}/{state["capability"]}/vnc.html?{query}'


def main():
    root = Path(sys.argv[1]).resolve()
    state = json.loads((root / "viewer.json").read_text())
    url = viewer_url(state)
    executable = Path(os.environ.get("PREFIX", "/data/data/com.termux/files/usr")) / "bin/termux-am-socket"
    # This binary accepts one command string; separate argv elements are refused.
    result = subprocess.run([str(executable), "start -a android.intent.action.VIEW -d " + url],
                            capture_output=True, text=True, timeout=20)
    proof = {"time": time.time(), "exit": result.returncode,
             "stderr_empty": not result.stderr, "activity_started": "Starting:" in result.stdout,
             "activity_error": "Error:" in result.stdout or "Error:" in result.stderr}
    # Never record capability URLs or browser authentication.
    with (root / "viewer-open-proof.jsonl").open("a") as output:
        output.write(json.dumps(proof) + "\n")
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
