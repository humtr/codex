#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'legacy-store-migration: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/legacy-store-migration-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

manager_dir="$FIXTURE_ROOT/native/manager"
legacy_store="$FIXTURE_ROOT/state/store"
legacy_runtime="$legacy_store/runtime/valid-runtime"
legacy_runtime_two="$legacy_store/runtime/valid-runtime-two"
legacy_raw="$legacy_store/raw/valid-raw"
bad_runtime="$legacy_store/runtime/bad-runtime"
bad_raw="$legacy_store/raw/bad-raw"
missing_runtime="$legacy_store/runtime/missing-runtime-entry"
missing_raw="$legacy_store/raw/missing-raw-entry"
mkdir -p "$manager_dir" "$legacy_runtime/codex-resources" "$legacy_runtime/codex-path" \
    "$legacy_runtime_two/codex-resources" "$legacy_runtime_two/codex-path" \
    "$legacy_raw/vendor/aarch64-unknown-linux-musl/bin" \
    "$bad_runtime" "$bad_raw/vendor/aarch64-unknown-linux-musl/bin" \
    "$missing_runtime" "$missing_raw/vendor/aarch64-unknown-linux-musl/bin" "$FIXTURE_ROOT/state"
cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
chmod 755 "$manager_dir/build-runtime.py" "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"
printf 'valid /proc/self/fd/33 runtime\n' >"$legacy_runtime/codex"
printf 'valid /proc/self/fd/33 runtime\n' >"$legacy_runtime_two/codex"
printf 'valid /etc/resolv.conf runtime\n' >"$legacy_raw/vendor/aarch64-unknown-linux-musl/bin/codex"
cp "$manager_dir/bwrap-termux-compat.py" "$legacy_runtime/codex-path/bwrap"
cp "$manager_dir/rg-termux-shim.sh" "$legacy_runtime/codex-path/rg"
printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-path/rg.real"
printf '{}\n' >"$legacy_runtime/codex-package.json"
cp -R "$legacy_runtime/codex-path/." "$legacy_runtime_two/codex-path/"
printf '{}\n' >"$legacy_runtime_two/codex-package.json"
printf 'bad runtime\n' >"$bad_runtime/codex"
printf 'bad raw\n' >"$bad_raw/vendor/aarch64-unknown-linux-musl/bin/codex"
chmod 755 "$legacy_runtime/codex" "$legacy_raw/vendor/aarch64-unknown-linux-musl/bin/codex" \
    "$legacy_runtime/codex-path/bwrap" "$legacy_runtime/codex-path/rg" "$legacy_runtime/codex-path/rg.real" \
    "$legacy_runtime_two/codex" "$legacy_runtime_two/codex-path/bwrap" \
    "$legacy_runtime_two/codex-path/rg" "$legacy_runtime_two/codex-path/rg.real" \
    "$bad_runtime/codex" "$bad_raw/vendor/aarch64-unknown-linux-musl/bin/codex"

builder_sha="$(sha256sum "$manager_dir/build-runtime.py" | awk '{print $1}')"
valid_runtime_sha="$(sha256sum "$legacy_runtime/codex" | awk '{print $1}')"
valid_raw_sha="$(sha256sum "$legacy_raw/vendor/aarch64-unknown-linux-musl/bin/codex" | awk '{print $1}')"
bad_runtime_sha="$(sha256sum "$bad_runtime/codex" | awk '{print $1}')"
bad_raw_sha="$(sha256sum "$bad_raw/vendor/aarch64-unknown-linux-musl/bin/codex" | awk '{print $1}')"
python3 - "$legacy_runtime/runtime-build.json" "$legacy_runtime_two/runtime-build.json" "$bad_runtime/runtime-build.json" \
    "$builder_sha" "$valid_runtime_sha" "$valid_raw_sha" "$bad_runtime_sha" "$bad_raw_sha" \
    "$FIXTURE_ROOT/state/registry.json" "$legacy_runtime" "$legacy_runtime_two" "$legacy_raw" "$bad_runtime" "$bad_raw" <<'PY'
import json, sys
from pathlib import Path

valid_manifest, valid_manifest_two, bad_manifest = map(Path, sys.argv[1:4])
builder_sha, valid_runtime_sha, valid_raw_sha, bad_runtime_sha, bad_raw_sha = sys.argv[4:9]
registry = Path(sys.argv[9])
valid_runtime, valid_runtime_two, valid_raw, bad_runtime, bad_raw = map(Path, sys.argv[10:15])
valid_data = {
    "patch_policy": "dns-fd33-only-v1",
    "builder_sha256": builder_sha,
    "runtime_sha256": valid_runtime_sha,
    "raw_sha256": valid_raw_sha,
}
valid_manifest.write_text(json.dumps(valid_data) + "\n")
valid_manifest_two.write_text(json.dumps(valid_data) + "\n")
bad_manifest.write_text(json.dumps({
    "patch_policy": "dns-fd33-only-v1",
    "builder_sha256": builder_sha,
    "runtime_sha256": "wrong",
    "raw_sha256": bad_raw_sha,
}) + "\n")
registry.write_text(json.dumps({
    "schema": 3,
    "active_tuple_id": "valid-tuple",
    "verified_tuple_id": "valid-tuple",
    "installs": [
        {
            "tuple_id": "valid-tuple",
            "raw_id": "valid-raw-id",
            "runtime_path": str(valid_runtime),
            "raw_path": str(valid_raw),
            "runtime_sha256": valid_runtime_sha,
            "raw_sha256": valid_raw_sha,
        },
        {
            "tuple_id": "bad-tuple",
            "raw_id": "bad-raw-id",
            "runtime_path": str(bad_runtime),
            "raw_path": str(bad_raw),
            "runtime_sha256": bad_runtime_sha,
            "raw_sha256": bad_raw_sha,
        },
        {
            "tuple_id": "valid-tuple-two",
            "raw_id": "valid-raw-id",
            "runtime_path": str(valid_runtime_two),
            "raw_path": str(valid_raw),
            "runtime_sha256": valid_runtime_sha,
            "raw_sha256": valid_raw_sha,
        },
        {
            "tuple_id": "missing-runtime-entry",
            "raw_id": "valid-raw-id",
            "runtime_path": str(valid_runtime),
            "raw_path": str(valid_raw),
            "runtime_sha256": valid_runtime_sha,
            "raw_sha256": valid_raw_sha,
        },
        {
            "tuple_id": "missing-raw-entry-tuple",
            "raw_id": "missing-raw-entry",
            "runtime_path": str(valid_runtime),
            "raw_path": str(valid_raw),
            "runtime_sha256": valid_runtime_sha,
            "raw_sha256": valid_raw_sha,
        },
    ],
    "runtime": {
        "valid-tuple": {"path": str(valid_runtime)},
        "valid-tuple-two": {"path": str(valid_runtime_two)},
        "bad-tuple": {"path": str(bad_runtime)},
        "missing-raw-entry-tuple": {"path": str(valid_runtime)},
    },
    "raw": {
        "valid-raw-id": {"path": str(valid_raw)},
        "bad-raw-id": {"path": str(bad_raw)},
    },
    "wrapper": {},
}) + "\n")
PY

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_STATE_DIR="$FIXTURE_ROOT/state"
export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
export CODEX_NATIVE_REGISTRY_FILE="$CODEX_NATIVE_STATE_DIR/registry.json"
export CODEX_NATIVE_STORE_DIR="$FIXTURE_ROOT/native/store"
export CODEX_NATIVE_LEGACY_STORE_DIR="$legacy_store"
export CODEX_NATIVE_STORE_MIGRATION_REPORT="$CODEX_NATIVE_STATE_DIR/legacy-store-migration.json"
export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

codex_with_lock codex_migrate_legacy_store_cache_unlocked || fail "legacy store migration failed"
python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_STORE_MIGRATION_REPORT" \
    "$CODEX_NATIVE_RUNTIME_STORE_DIR/valid-runtime" "$CODEX_NATIVE_RAW_STORE_DIR/valid-raw" \
    "$CODEX_NATIVE_RUNTIME_STORE_DIR/valid-runtime-two" "$bad_runtime" <<'PY'
import json, sys
from pathlib import Path

registry_path, report_path, runtime_target, raw_target, runtime_target_two, bad_runtime = map(Path, sys.argv[1:7])
registry = json.loads(registry_path.read_text())
report = json.loads(report_path.read_text())
assert registry["runtime"]["valid-tuple"]["path"] == str(runtime_target)
assert registry["runtime"]["valid-tuple-two"]["path"] == str(runtime_target_two)
assert registry["raw"]["valid-raw-id"]["path"] == str(raw_target)
assert registry["runtime"]["bad-tuple"]["path"] == str(bad_runtime)
assert report["imported"] == ["valid-tuple", "valid-tuple-two"]
assert report["skipped"][0]["tuple_id"] == "bad-tuple"
assert {item["tuple_id"] for item in report["skipped"]} == {
    "bad-tuple",
    "missing-runtime-entry",
    "missing-raw-entry-tuple",
}
PY
[ -d "$CODEX_NATIVE_RUNTIME_STORE_DIR/valid-runtime" ] || fail "valid runtime was not imported"
[ -d "$CODEX_NATIVE_RAW_STORE_DIR/valid-raw" ] || fail "valid raw was not imported"
[ ! -e "$CODEX_NATIVE_RUNTIME_STORE_DIR/bad-runtime" ] || fail "invalid runtime was imported"

report_before="$(sha256sum "$CODEX_NATIVE_STORE_MIGRATION_REPORT")"
codex_with_lock codex_migrate_legacy_store_cache_unlocked || fail "repeated legacy store migration failed"
[ "$(sha256sum "$CODEX_NATIVE_STORE_MIGRATION_REPORT")" = "$report_before" ] \
    || fail "completed migration report changed on repeated migration"

malformed_root="$(mktemp -d "$FIXTURE_PARENT/legacy-store-migration-malformed.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT" "$malformed_root"' EXIT
mkdir -p "$malformed_root/state/store/runtime" "$malformed_root/state"
printf '{}\n' >"$malformed_root/state/registry.json"
if codex_native_cmd legacy-store-migrate \
    --legacy-store-dir "$malformed_root/state/store" \
    --runtime-store-dir "$malformed_root/native/store/runtime" \
    --raw-store-dir "$malformed_root/native/store/raw" \
    --registry-file "$malformed_root/state/registry.json" \
    --runtime-builder "$manager_dir/build-runtime.py" \
    --manager-dir "$manager_dir" \
    --patch-policy "dns-fd33-only-v1" \
    --report-file "$malformed_root/state/report.json" \
    --completed-at "2026-06-15T00:00:00Z" >/dev/null 2>&1; then
    fail "malformed legacy registry unexpectedly succeeded"
fi
[ ! -e "$malformed_root/state/report.json" ] || fail "malformed legacy registry wrote a false success report"

report_fail_root="$(mktemp -d "$FIXTURE_PARENT/legacy-store-migration-report-fail.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT" "$malformed_root" "$report_fail_root"' EXIT
mkdir -p "$report_fail_root/state/store/runtime" "$report_fail_root/state/report.json"
printf '{"schema":3,"installs":[],"runtime":{},"raw":{},"wrapper":{}}\n' >"$report_fail_root/state/registry.json"
if codex_native_cmd legacy-store-migrate \
    --legacy-store-dir "$report_fail_root/state/store" \
    --runtime-store-dir "$report_fail_root/native/store/runtime" \
    --raw-store-dir "$report_fail_root/native/store/raw" \
    --registry-file "$report_fail_root/state/registry.json" \
    --runtime-builder "$manager_dir/build-runtime.py" \
    --manager-dir "$manager_dir" \
    --patch-policy "dns-fd33-only-v1" \
    --report-file "$report_fail_root/state/report.json" \
    --completed-at "2026-06-15T00:00:00Z" >/dev/null 2>&1; then
    fail "report write failure unexpectedly succeeded"
fi

printf 'legacy-store-migration: ok\n'
