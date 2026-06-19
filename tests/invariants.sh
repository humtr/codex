#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'invariants: FAIL: %s\n' "$*" >&2
    exit 1
}

cd "$ROOT_DIR"

forbidden_terms=(
    "codex""_native"
    "CODEX""_NATIVE"
    "codex/""native"
    "codex"" ""native"
    "native"".lock"
    "CODEX_TERMUX""_RESOLVER_FD"
    "CODEX_TERMUX""_SHARED_PLUGINS_DIR"
    "codex_profile""_share_plugins"
)
for term in "${forbidden_terms[@]}"; do
    if grep -RIn --exclude-dir=.git --exclude-dir=__pycache__ --exclude='*.pyc' -- "$term" . >"$TMP_DIR/forbidden" 2>/dev/null; then
        cat "$TMP_DIR/forbidden" >&2
        fail "forbidden legacy contract term remains: $term"
    fi
done

grep -Fx 'CODEX_TERMUX_WRAPPER_VERSION=1.2.1' config/wrapper-version.env >/dev/null \
    || fail 'wrapper version mismatch'
grep -Fx 'CODEX_TERMUX_WRAPPER_CHANNEL=slim' config/wrapper-version.env >/dev/null \
    || fail 'wrapper channel mismatch'
grep -Fx 'CODEX_TERMUX_WRAPPER_REPO=local/codex-termux' config/wrapper-version.env >/dev/null \
    || fail 'wrapper repo mismatch'

grep -F 'PYTHONDONTWRITEBYTECODE=1' lib/codex-termux.sh >/dev/null \
    || fail 'helper bytecode suppression missing'
grep -F 'python3 -B -m codex_termux.cli' lib/codex-termux.sh >/dev/null \
    || fail 'helper -B invocation missing'
grep -F 'codex_profile_list_command()' lib/codex-termux.sh >/dev/null \
    || fail 'profile list command helper missing'
grep -F 'list|ls)' lib/codex-termux.sh >/dev/null \
    || fail 'profile list dispatch missing'
grep -F '33<"$CODEX_TERMUX_RESOLV_CONF"' lib/codex-termux.sh >/dev/null \
    || fail 'runtime fd33 launcher contract missing'
grep -F 'PATCH_POLICY = "dns-fd33-only-v1"' tools/build-runtime.py >/dev/null \
    || fail 'builder patch policy changed'
grep -F 'RESOLV_CONF_TARGET = b"/proc/self/fd/33"' tools/build-runtime.py >/dev/null \
    || fail 'builder resolver target changed'

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B -m codex_termux.cli validate --root "$ROOT_DIR" >/dev/null

bytecode_found="$(find . \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -print -quit 2>/dev/null || true)"
[ -z "$bytecode_found" ] || fail "bytecode artifact found: $bytecode_found"

stale_repo_terms=(
    "\"local/""codex\""
    "local/""codex\\n"
)
for term in "${stale_repo_terms[@]}"; do
    if grep -RIn --exclude-dir=.git --exclude-dir=__pycache__ --exclude='*.pyc' -- "$term" . >"$TMP_DIR/repo-metadata" 2>/dev/null; then
        cat "$TMP_DIR/repo-metadata" >&2
        fail "stale wrapper repo metadata remains: $term"
    fi
done

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B - <<'PYTHON'
from pathlib import Path
from tempfile import TemporaryDirectory
from codex_termux import registry

with TemporaryDirectory() as tmp:
    root = Path(tmp)
    registry_file = root / "registry.json"
    runtime_store = root / "runtime-store"
    runtime_path = runtime_store / "runtime"
    raw_path = root / "raw-store" / "raw"
    tuple_id = registry.record(
        registry_file=registry_file,
        version="0.0.0-linux-arm64",
        raw_sha256="a" * 64,
        runtime_sha256="b" * 64,
        package_spec="@openai/codex@0.0.0-linux-arm64",
        runtime_path=str(runtime_path),
        wrapper_version="1.2.1",
        wrapper_commit="testcommit",
        runtime_store_dir=runtime_store,
        updated_at="2026-01-01T00:00:00+00:00",
        smoke_tested_at="2026-01-01T00:00:00+00:00",
        raw_path=str(raw_path),
    )
    data = registry.load(registry_file)
    wrapper_id = data["runtime"][tuple_id]["wrapper_id"]
    assert data["wrapper"][wrapper_id]["repo"] == "local/codex-termux"
PYTHON

printf 'invariants: ok\n'
