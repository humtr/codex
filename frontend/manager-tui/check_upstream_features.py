"""Qualify the 0.162 feature surfaces retained by the 0.163 upgrade."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

from check_command_boundary import nonzero_tests

# Run upstream's actual tests through the assembled frontend/unchanged engine.
# Every topic must have an executed PASS, not merely a nonempty aggregate suite.
SUITES = {
    "tui": ("codex-tui", "--lib", {
        "worktrees": "managed_worktree_tools::tests::",
        "pinning": "pin_action_updates_shared_section_state",
        "copy": "slash_copy",
        "copy-shortcut": "copy_shortcut",
        "scroll": "mouse_scroll_speed",
        "links": "terminal_hyperlinks",
        "defaults": "fresh_startup_uses_server_defaults_with_explicit_and_managed_precedence",
        "timezone": "absolute_time_uses_child_timezone",
    }),
    "engine": ("codex-core", "--test=all", {
        "provider-capabilities": "custom_provider_capabilities_constrain_web_access_and_enable_compaction",
        "settled-helpers": "code_mode_settled_helpers_report_successes_and_failures",
        "tool-search": "code_mode_catalog_messages",
        "retry-after": "retry_after::",
    }),
    "patch": ("codex-apply-patch", "--test=all", {
        "crlf": "test_apply_patch_cli_preserves_crlf_from_target_file",
        "mixed-endings": "test_apply_patch_cli_preserves_untouched_mixed_line_endings",
    }),
}


def verify(logs):
    result = {}
    for suite, (_, _, topics) in SUITES.items():
        path = logs / f"profile-tui-upstream-{suite}.log"
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 16 * 1024 * 1024:
            raise ValueError("invalid upstream feature proof: " + suite)
        log = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', path.read_text())
        count = nonzero_tests(log)
        passed = re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+(.+)$', log, re.M)
        missing = [name for name, fragment in topics.items()
                   if not any(fragment in test for test in passed)]
        if missing:
            raise ValueError("upstream feature topics did not execute: " + ", ".join(missing))
        result[suite] = {"passed": count, "topics": list(topics)}
    return result


def run(source, logs):
    logs.mkdir(parents=True, exist_ok=True)
    for suite, (package, target, topics) in SUITES.items():
        query = " | ".join(f"test({fragment})" for fragment in topics.values())
        command = ["just", "test", "--retries", "0", "--status-level", "pass", "--locked", "--target",
                   "aarch64-unknown-linux-musl", "-p", package, target, "-E", query]
        path = logs / f"profile-tui-upstream-{suite}.log"
        with path.open("w") as log:
            process = subprocess.Popen(command, cwd=source, stdout=subprocess.PIPE,
                                       stderr=subprocess.STDOUT, text=True)
            for line in process.stdout:
                sys.stdout.write(line)
                log.write(line)
            if process.wait():
                raise ValueError("upstream feature suite failed: " + suite)
    return verify(logs)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--logs", type=Path, required=True)
    parser.add_argument("--run", type=Path, help="assembled official checkout")
    args = parser.parse_args()
    try:
        print(json.dumps(run(args.run, args.logs) if args.run else verify(args.logs), sort_keys=True))
    except (OSError, ValueError) as error:
        parser.exit(1, "upstream feature qualification refused: " + str(error) + "\n")
