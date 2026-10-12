#!/usr/bin/env python3
"""Exercise a candidate Manager against real Android notification providers.

Only synthetic question text and disposable Manager state are used. The script
removes its own notifications, never opens a terminal and emits aggregate proof.
Run deliberately on Termux; this is not an unattended unit-test provider.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid


def qualify(manager, core, parent):
    group = "codex-proof-" + uuid.uuid4().hex[:12]
    ids = [str(uuid.uuid4()), str(uuid.uuid4())]
    notification_ids = [f"codex:{group}:{identity}" for identity in ids]
    with tempfile.TemporaryDirectory(prefix="notify-delivery-", dir=parent) as temporary:
        home = Path(temporary)
        home.chmod(0o700)
        prefix = os.environ["PREFIX"]
        env = {
            "HOME": str(home), "PREFIX": prefix, "TMPDIR": str(parent),
            "PATH": f"{prefix}/bin", "LANG": "C.UTF-8",
            "CODEX_TERMUX_CORE_API": "codex-manager-core-v1",
            "CODEX_TERMUX_CORE_ENTRYPOINT": str(core),
        }

        def run(*args, payload=None):
            result = subprocess.run([str(manager), *args], env=env,
                                    input=json.dumps(payload).encode() if payload else b"",
                                    capture_output=True, timeout=20)
            assert result.returncode == 0, "candidate Manager failed"
            assert not result.stderr, "candidate Manager emitted unexpected diagnostics"

        def notifications():
            result = subprocess.run(["termux-notification-list"], capture_output=True, timeout=20)
            assert result.returncode == 0, "Android notification listing failed"
            return [item for item in json.loads(result.stdout)
                    if item.get("packageName") == "com.termux.api"
                    and item.get("group") == group and item.get("content")]

        def wait_expected(expected):
            deadline = time.monotonic() + 5
            while True:
                visible = notifications()
                if {item.get("tag"): item["content"] for item in visible} == expected and len(visible) == len(expected):
                    return visible
                assert time.monotonic() < deadline, "Android notification state did not converge"
                time.sleep(0.1)

        run("notify", "set", "--channel", "notification", "--hooks", "UserInputRequest,Stop",
            "--group", group, "--focus", "termux")
        try:
            for name, field, body in [
                ("request_user_input", "question", "Synthetic first question?"),
                ("functions.request_user_input_async", "title", "Synthetic updated question?"),
            ]:
                run("notify", "emit", "UserInputRequest", payload={
                    "title": "알림 통합 검증", "session_id": ids[0], "tool_name": name,
                    "tool_input": {"questions": [{field: body, "options": ["excluded"]}]},
                })
                visible = wait_expected({notification_ids[0]: body})
                assert visible[0].get("tag") == notification_ids[0], "Android identity differs from Manager identity"
            completion = {"type": "agent-turn-complete", "thread-id": ids[0],
                          "title": "알림 통합 검증", "last-assistant-message": "Synthetic question context retained."}
            run("notify", "emit", "Stop", json.dumps(completion))
            wait_expected({notification_ids[0]: completion["last-assistant-message"]})
            first_completion = completion["last-assistant-message"]
            completion["thread-id"] = ids[1]
            completion["last-assistant-message"] = "Synthetic separate conversation."
            run("notify", "emit", "Stop", json.dumps(completion))
            expected = {notification_ids[0]: first_completion, notification_ids[1]: completion["last-assistant-message"]}
            wait_expected(expected)
            run("notify", "emit", "UserInputRequest", payload={"session_id": ids[0]})
            wait_expected(expected)
        finally:
            for identity in notification_ids:
                subprocess.run(["termux-notification-remove", identity], capture_output=True, timeout=20, check=True)
        wait_expected({})
    return {"questionBodyAccepted": True, "completionUpdatesSameNotification": True,
            "separateConversationsPreserved": True, "emptyReminderSuppressed": True,
            "ownedNotificationsRemoved": True, "installedStateModified": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manager", type=Path, required=True)
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--parent", type=Path, default=Path(os.environ.get("TMPDIR", "/tmp")))
    parser.add_argument("--proof", type=Path, required=True)
    args = parser.parse_args()
    proof = qualify(args.manager.resolve(), args.core.resolve(), args.parent)
    args.proof.write_text(json.dumps(proof, indent=2) + "\n")
    args.proof.chmod(0o600)
    print("Android notification delivery: question/completion one notification, separate conversations preserved, trial removed")


if __name__ == "__main__":
    main()
