#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'runtime-build: FAIL: %s\n' "$*" >&2
    exit 1
}

raw_vendor="$TMP_DIR/raw/vendor/aarch64-unknown-linux-musl"
runtime_dir="$TMP_DIR/runtime"
mkdir -p "$raw_vendor/bin" "$raw_vendor/codex-resources/zsh/bin" "$raw_vendor/codex-path"
printf 'upstream-only\n' >"$raw_vendor/upstream-only.txt"
cat >"$raw_vendor/bin/codex" <<'SCRIPT'
#!/bin/sh
# /etc/resolv.conf
# resolver fallback /etc/resolv.conf
# /etc/codex/config.toml
# /etc/codex/requirements.toml
# /etc/codex/managed_config.toml
[ "${1:-}" = "--version" ] && printf 'codex test\n'
exit 0
SCRIPT
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/bin/codex-code-mode-host"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-resources/bwrap"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-resources/zsh/bin/zsh"
printf '#!/bin/sh\nexit 0\n' >"$raw_vendor/codex-path/rg"
printf '{"name":"@openai/codex"}\n' >"$raw_vendor/codex-package.json"
chmod 755 "$raw_vendor/bin/codex" "$raw_vendor/bin/codex-code-mode-host" \
    "$raw_vendor/codex-resources/bwrap" \
    "$raw_vendor/codex-resources/zsh/bin/zsh" "$raw_vendor/codex-path/rg"

PYTHONDONTWRITEBYTECODE=1 python3 -B "$ROOT_DIR/tools/build-runtime.py" "$raw_vendor" \
    --runtime-dir "$runtime_dir" >"$TMP_DIR/build-report.json"

PYTHONDONTWRITEBYTECODE=1 python3 -B - "$ROOT_DIR" "$raw_vendor/bin/codex" "$runtime_dir" <<'PYTHON'
import hashlib
import json
import os
import sys
from pathlib import Path

root = Path(sys.argv[1])
raw = Path(sys.argv[2])
runtime_dir = Path(sys.argv[3])
runtime = runtime_dir / "codex"
raw_host = raw.parent / "codex-code-mode-host"
runtime_host = runtime_dir / "codex-code-mode-host"
manifest_path = runtime_dir / "runtime-build.json"
raw_bytes = raw.read_bytes()
runtime_bytes = runtime.read_bytes()
raw_host_bytes = raw_host.read_bytes()
rewrites = {
    b"/etc/resolv.conf": b"/proc/self/fd/33",
    b"/etc/codex/config.toml": b"/dev/fd/34/config.toml",
    b"/etc/codex/requirements.toml": b"/dev/fd/34/requirements.toml",
    b"/etc/codex/managed_config.toml": b"/dev/fd/34/managed_config.toml",
}
expected = raw_bytes
for source, target in rewrites.items():
    expected = expected.replace(source, target)
assert runtime_bytes == expected, "runtime binary is not exactly the Termux fd remap patch"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
assert manifest["patch_policy"] == "termux-fd-remap-v1"
assert manifest["raw_sha256"] == hashlib.sha256(raw_bytes).hexdigest()
assert manifest["runtime_sha256"] == hashlib.sha256(runtime_bytes).hexdigest()
assert manifest["code_mode_host_sha256"] == hashlib.sha256(raw_host_bytes).hexdigest()
assert manifest["upstream_tree_sha256"]
assert manifest["builder_sha256"] == hashlib.sha256((root / "tools/build-runtime.py").read_bytes()).hexdigest()
assert runtime_host.read_bytes() == raw_host_bytes, "code-mode host must be copied without patching"
assert os.access(runtime_host, os.X_OK), "code-mode host must be executable"
assert (runtime_dir / "upstream/upstream-only.txt").read_text() == "upstream-only\n"
for source, target in rewrites.items():
    entry = manifest["rewrites"][source.decode("ascii")]
    expected_count = raw_bytes.count(source)
    assert entry["source_count"] == expected_count
    assert entry["target_count_after"] == expected_count
    assert target in runtime_bytes
for rel in (
    "codex",
    "codex-code-mode-host",
    "codex-resources/bwrap",
    "codex-resources/zsh/bin/zsh",
    "codex-path/bwrap",
    "codex-path/rg",
    "codex-path/rg.real",
    "codex-package.json",
    "runtime-build.json",
    "upstream/upstream-only.txt",
):
    path = runtime_dir / rel
assert path.exists(), f"missing runtime entry: {rel}"
PYTHON

if PYTHONDONTWRITEBYTECODE=1 python3 -B "$ROOT_DIR/tools/bwrap-termux-compat.py" -- /definitely/missing/codex-bwrap-test 2>"$TMP_DIR/bwrap.err"; then
    fail 'bwrap compat accepted a missing executable'
fi
grep -F "failed to exec" "$TMP_DIR/bwrap.err" >/dev/null \
    || fail 'bwrap compat failure did not include exec context'

printf 'runtime-build: ok\n'
