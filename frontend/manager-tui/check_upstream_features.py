"""Qualify the 0.162 feature surfaces retained by the 0.163 upgrade."""
import argparse
import hashlib
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


def reusable_frontend(artifact, current_patch, metadata, repository, upstream, version):
    """Reuse an identified executable only with identical prepared production inputs."""
    fields = dict(line.split('=', 1) for line in (artifact / 'source.txt').read_text().splitlines())
    if (metadata['repository']['full_name'] != repository or
            metadata['head_repository']['full_name'] != repository or
            metadata['path'] != '.github/workflows/manager-tui-native.yml' or
            metadata['status'] != 'completed' or metadata['head_sha'] != fields['source_sha'] or
            fields['upstream_sha'] != upstream or fields['target'] != 'aarch64-unknown-linux-musl' or
            fields['profile'] != 'release opt-level=z codegen-units=1 strip=symbols' or
            (artifact / 'version.txt').read_text() != f'codex-cli {version}\n'):
        raise ValueError('frontend build identity does not match the qualification')
    def production(patch):
        result = []
        for section in re.split(r'(?m)(?=^diff --git )', patch):
            if not section:
                continue
            match = re.match(r'diff --git a/(\S+) b/\1\n', section)
            if match is None:
                raise ValueError('unrecognized prepared source diff')
            if not match[1].startswith('codex-rs/app-server/tests/'):
                result.append(section)
        return ''.join(result)
    if production((artifact / 'native.patch').read_text()) != production(current_patch.read_text()):
        raise ValueError('prepared production inputs changed; build the new frontend')
    binary = artifact / 'codex'
    with binary.open('rb') as source:
        actual = hashlib.file_digest(source, 'sha256').hexdigest()
    if actual != (artifact / 'sha256.txt').read_text().split()[0]:
        raise ValueError('frontend executable differs from its build output')
    return binary


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
