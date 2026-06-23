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

grep -Fx 'CODEX_TERMUX_WRAPPER_VERSION=20260623-1' config/wrapper-version.env >/dev/null \
    || fail 'wrapper version mismatch'
grep -Fx 'CODEX_TERMUX_WRAPPER_CHANNEL=termux' config/wrapper-version.env >/dev/null \
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
        wrapper_version="20260623-1",
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

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools python3 -B - <<'PYTHON'
import contextlib
import hashlib
import io
import json
from pathlib import Path
from tempfile import TemporaryDirectory

from codex_termux import registry, use


def make_runtime(path: Path, builder: Path, runtime_bytes: bytes) -> str:
    path.mkdir(parents=True)
    runtime = path / "codex"
    runtime.write_bytes(runtime_bytes)
    runtime_sha = hashlib.sha256(runtime_bytes).hexdigest()
    manifest = {
        "patch_policy": "dns-fd33-only-v1",
        "builder_sha256": hashlib.sha256(builder.read_bytes()).hexdigest(),
        "runtime_sha256": runtime_sha,
    }
    (path / "runtime-build.json").write_text(json.dumps(manifest) + "\n", encoding="utf-8")
    return runtime_sha


def make_raw(path: Path, raw_bytes: bytes) -> str:
    raw_bin = path / "vendor/aarch64-unknown-linux-musl/bin/codex"
    raw_bin.parent.mkdir(parents=True)
    raw_bin.write_bytes(raw_bytes)
    return hashlib.sha256(raw_bytes).hexdigest()


with TemporaryDirectory() as tmp:
    root = Path(tmp)
    registry_file = root / "registry.json"
    store_root = root / "store"
    runtime_store = store_root / "runtime"
    raw_store = store_root / "raw"
    builder = root / "build-runtime.py"
    builder.write_text("#!/usr/bin/env python3\n", encoding="utf-8")

    runtime_a = runtime_store / "runtime-a"
    raw_a = raw_store / "raw-a"
    runtime_a_sha = make_runtime(runtime_a, builder, b"runtime-a")
    raw_a_sha = make_raw(raw_a, b"raw-a")
    active_tuple = registry.record(
        registry_file=registry_file,
        version="0.142.0-linux-arm64",
        raw_sha256=raw_a_sha,
        runtime_sha256=runtime_a_sha,
        package_spec="@openai/codex@0.142.0-linux-arm64",
        runtime_path=str(runtime_a),
        wrapper_version="20260619-1",
        wrapper_commit="aaaaaaa11111",
        runtime_store_dir=runtime_store,
        updated_at="2026-06-19T00:00:00+00:00",
        smoke_tested_at="2026-06-19T00:00:00+00:00",
        raw_path=str(raw_a),
    )
    registry.record(
        registry_file=registry_file,
        version="0.142.0-linux-arm64",
        raw_sha256=raw_a_sha,
        runtime_sha256=runtime_a_sha,
        package_spec="@openai/codex@0.142.0-linux-arm64",
        runtime_path=str(runtime_a),
        wrapper_version="20260623-1",
        wrapper_commit="bbbbbbb22222",
        runtime_store_dir=runtime_store,
        updated_at="2026-06-23T00:00:00+00:00",
        smoke_tested_at="2026-06-23T00:00:00+00:00",
        raw_path=str(raw_a),
    )
    registry.activate_existing_tuple(registry_file, active_tuple)

    runtime_b = runtime_store / "runtime-b"
    raw_b = raw_store / "raw-b"
    runtime_b_sha = make_runtime(runtime_b, builder, b"runtime-b")
    raw_b_sha = make_raw(raw_b, b"raw-b")
    registry.record(
        registry_file=registry_file,
        version="0.141.0-linux-arm64",
        raw_sha256=raw_b_sha,
        runtime_sha256=runtime_b_sha,
        package_spec="@openai/codex@0.141.0-linux-arm64",
        runtime_path=str(runtime_b),
        wrapper_version="20260619-1",
        wrapper_commit="ccccccc33333",
        runtime_store_dir=runtime_store,
        updated_at="2026-06-19T12:00:00+00:00",
        smoke_tested_at="2026-06-19T12:00:00+00:00",
        raw_path=str(raw_b),
    )

    runtime_c = runtime_store / "runtime-c"
    raw_c = raw_store / "raw-c"
    runtime_c_sha = make_runtime(runtime_c, builder, b"runtime-c")
    raw_c_sha = make_raw(raw_c, b"raw-c")
    registry.record(
        registry_file=registry_file,
        version="0.141.0-linux-arm64",
        raw_sha256=raw_c_sha,
        runtime_sha256=runtime_c_sha,
        package_spec="@openai/codex@0.141.0-linux-arm64",
        runtime_path=str(runtime_c),
        wrapper_version="20260619-2",
        wrapper_commit="ddddddd44444",
        runtime_store_dir=runtime_store,
        updated_at="2026-06-19T18:00:00+00:00",
        smoke_tested_at="2026-06-19T18:00:00+00:00",
        raw_path=str(raw_c),
    )
    registry.activate_existing_tuple(registry_file, active_tuple)

    rows = use.runtime_rows_from_registry(
        registry_file=registry_file,
        latest="0.142.0-linux-arm64",
        runtime_store_dir=runtime_store,
        runtime_builder=builder,
        patch_policy="dns-fd33-only-v1",
    )
    cached_rows = [row for row in rows if row["kind"] == "cached"]
    assert len(cached_rows) == 3, rows
    active_rows = [row for row in cached_rows if row.get("active") == "1"]
    assert len(active_rows) == 1, rows
    assert active_rows[0]["wrapper_commit"] == "aaaaaaa11111", active_rows[0]

    latest_row, remaining = registry.menu_rows(
        registry_file=registry_file,
        latest="0.142.0-linux-arm64",
        runtime_store_dir=runtime_store,
        runtime_builder=builder,
        patch_policy="dns-fd33-only-v1",
    )
    assert latest_row is None
    assert len(remaining) == 3

    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        use.render_runtime_rows(rows, mode="menu", interactive_limit=0)
    menu = stderr.getvalue()
    assert "0.142.0 (2026-06-19)" in menu, menu
    assert "0.141.0 (2026-06-19 · 2026-06-19 (r1))" in menu, menu
    assert "0.141.0 (2026-06-19 · 2026-06-19 (r2))" in menu, menu
PYTHON

printf 'invariants: ok\n'
