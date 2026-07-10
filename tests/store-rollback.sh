#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'store-rollback: FAIL: %s\n' "$*" >&2
    exit 1
}

shell_bin="$(command -v bash)"
[ -n "$shell_bin" ] || fail 'bash not found'

raw_vendor="$TMP_DIR/raw-source/vendor/aarch64-unknown-linux-musl"
runtime_source="$TMP_DIR/runtime-source"
root_dir="$TMP_DIR/home/.local/lib/codex/termux"
state_dir="$TMP_DIR/home/.local/share/codex/termux"
manager_dir="$root_dir/manager"
runtime_store="$root_dir/store/runtime"
raw_store="$root_dir/store/raw"
current_link="$root_dir/current"
verified_link="$root_dir/verified"
raw_link="$root_dir/raw"
state_file="$state_dir/state.json"
registry_file="$state_dir/registry.json"
resolv_conf="$TMP_DIR/prefix/etc/resolv.conf"
cert_file="$TMP_DIR/prefix/etc/tls/cert.pem"
cert_dir="$TMP_DIR/prefix/etc/tls/certs"

mkdir -p "$raw_vendor/bin" "$raw_vendor/codex-resources/zsh/bin" "$raw_vendor/codex-path" \
    "$manager_dir" "$runtime_store" "$raw_store" "$state_dir" \
    "$(dirname "$resolv_conf")" "$(dirname "$cert_file")" "$cert_dir"
printf 'upstream-only\n' >"$raw_vendor/upstream-only.txt"
cat >"$raw_vendor/bin/codex" <<'SCRIPT'
#!/bin/sh
# /etc/resolv.conf
# /etc/codex/config.toml
# /etc/codex/requirements.toml
# /etc/codex/managed_config.toml
[ "${1:-}" = "--version" ] && printf 'codex good\n'
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

cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
cp "$ROOT_DIR/config/wrapper-version.env" "$manager_dir/wrapper-version.env"
{
    printf 'CODEX_TERMUX_WRAPPER_COMMIT=testcommit\n'
    printf 'CODEX_TERMUX_WRAPPER_INSTALLED_AT=2026-01-01T00:00:00+00:00\n'
} >>"$manager_dir/wrapper-version.env"
chmod 755 "$manager_dir/build-runtime.py" "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"
printf 'nameserver 127.0.0.1\n' >"$resolv_conf"
printf 'cert\n' >"$cert_file"

PYTHONDONTWRITEBYTECODE=1 python3 -B "$manager_dir/build-runtime.py" "$raw_vendor" \
    --runtime-dir "$runtime_source" >/dev/null

good_runtime="$runtime_store/good-runtime"
good_raw="$raw_store/good-raw"
cp -a "$runtime_source" "$good_runtime"
mkdir -p "$good_raw"
cp -a "$TMP_DIR/raw-source/vendor" "$good_raw/vendor"

bad_runtime="$runtime_store/bad-runtime"
cp -a "$good_runtime" "$bad_runtime"
printf 'drift\n' >>"$bad_runtime/codex"
chmod 755 "$bad_runtime/codex"
old_raw="$raw_store/old-raw"
cp -a "$good_raw" "$old_raw"

runtime_sha="$(sha256sum "$good_runtime/codex" | awk '{print $1}')"
raw_sha="$(sha256sum "$good_raw/vendor/aarch64-unknown-linux-musl/bin/codex" | awk '{print $1}')"
ln -s "$bad_runtime" "$current_link"
ln -s "$good_runtime" "$verified_link"
ln -s "$old_raw" "$raw_link"

PYTHONDONTWRITEBYTECODE=1 python3 -B - "$registry_file" "$state_file" "$good_runtime" "$good_raw" "$bad_runtime" "$old_raw" "$runtime_sha" "$raw_sha" <<'PYTHON'
import json
import sys
from pathlib import Path

registry_file, state_file, good_runtime, good_raw, bad_runtime, old_raw = map(Path, sys.argv[1:7])
runtime_sha, raw_sha = sys.argv[7:9]
version = "0.141.0-linux-arm64"
package_spec = "@openai/codex@0.141.0-linux-arm64"
raw_id = "raw-good"
wrapper_id = "wrapper-test"
tuple_id = f"{raw_id}__{wrapper_id}"
registry = {
    "schema": 3,
    "installs": [
        {
            "version": version,
            "raw_sha256": raw_sha,
            "runtime_sha256": runtime_sha,
            "package_spec": package_spec,
            "runtime_path": str(good_runtime),
            "raw_path": str(good_raw),
            "updated_at": "2026-01-01T00:00:00+00:00",
            "raw_id": raw_id,
            "wrapper_id": wrapper_id,
            "tuple_id": tuple_id,
        }
    ],
    "raw": {
        raw_id: {
            "version": version,
            "sha256": raw_sha,
            "package_spec": package_spec,
            "path": str(good_raw),
            "updated_at": "2026-01-01T00:00:00+00:00",
        }
    },
    "wrapper": {
        wrapper_id: {
            "version": "20260623-1",
            "commit": "testcommit",
            "repo": "local/codex-termux",
            "updated_at": "2026-01-01T00:00:00+00:00",
        }
    },
    "runtime": {
        tuple_id: {
            "raw_id": raw_id,
            "wrapper_id": wrapper_id,
            "runtime_sha256": runtime_sha,
            "path": str(good_runtime),
            "updated_at": "2026-01-01T00:00:00+00:00",
            "smoke_tested_at": "2026-01-01T00:00:00+00:00",
        }
    },
    "active_tuple_id": "bad-current",
    "verified_tuple_id": tuple_id,
}
state = {
    "schema": 3,
    "version": "bad",
    "raw_sha256": raw_sha,
    "runtime_sha256": runtime_sha,
    "package_spec": package_spec,
    "active_tuple_id": "bad-current",
    "wrapper_version": "20260623-1",
    "wrapper_commit": "oldcommit",
    "updated_at": "2026-01-01T00:00:00+00:00",
    "verified_tuple_id": tuple_id,
    "verified_at": "2026-01-01T00:00:00+00:00",
}
stale_tuple_id = "raw-stale__wrapper-stale"
registry["installs"].append(
    {
        "version": "0.140.0-linux-arm64",
        "raw_sha256": raw_sha,
        "runtime_sha256": runtime_sha,
        "package_spec": package_spec,
        "runtime_path": str(good_runtime.parent / "missing-runtime"),
        "raw_path": str(good_raw.parent / "missing-raw"),
        "updated_at": "2026-01-01T00:00:00+00:00",
        "raw_id": "raw-stale",
        "wrapper_id": "wrapper-stale",
        "tuple_id": stale_tuple_id,
    }
)
registry_file.write_text(json.dumps(registry, sort_keys=True) + "\n", encoding="utf-8")
state_file.write_text(json.dumps(state, sort_keys=True) + "\n", encoding="utf-8")
PYTHON

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B -m codex_termux.cli activation-restore-verified \
    --current-link "$current_link" \
    --verified-link "$verified_link" \
    --raw-link "$raw_link" \
    --state-file "$state_file" \
    --registry-file "$registry_file" \
    --runtime-store-dir "$runtime_store" \
    --raw-store-dir "$raw_store" \
    --wrapper-version 20260623-1 \
    --wrapper-commit testcommit \
    --updated-at 2026-01-01T00:00:00+00:00 \
    --shell-bin "$shell_bin" \
    --shell-lib "$ROOT_DIR/lib/codex-termux.sh" \
    --home "$TMP_DIR/home" \
    --prefix "$TMP_DIR/prefix" \
    --manager-dir "$manager_dir" \
    --runtime-builder "$manager_dir/build-runtime.py" \
    --resolv-conf "$resolv_conf" \
    --cert-file "$cert_file" \
    --cert-dir "$cert_dir" \
    --patch-policy termux-fd-remap-v1 >/dev/null

[ "$(readlink "$current_link")" = "$good_runtime" ] || fail 'current pointer was not restored to verified runtime'
[ "$(readlink "$verified_link")" = "$good_runtime" ] || fail 'verified pointer changed unexpectedly'
[ "$(readlink "$raw_link")" = "$good_raw" ] || fail 'raw pointer was not restored to verified raw'

for index in 1 2 3; do
    extra="$runtime_store/extra-$index"
    cp -a "$good_runtime" "$extra"
    touch -d "2026-01-0$index UTC" "$extra" 2>/dev/null || touch "$extra"
done
running_runtime="$runtime_store/running-runtime"
cp -a "$good_runtime" "$running_runtime"
touch -d "2025-12-31 UTC" "$running_runtime" 2>/dev/null || touch "$running_runtime"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B -m codex_termux.cli store-prune \
    --runtime-store-dir "$runtime_store" \
    --raw-store-dir "$raw_store" \
    --registry-file "$registry_file" \
    --state-file "$state_file" \
    --runtime-builder "$manager_dir/build-runtime.py" \
    --patch-policy termux-fd-remap-v1 \
    --retention 1 \
    --current-link "$current_link" \
    --verified-link "$verified_link" \
    --protect-runtime-path "$running_runtime" \
    --raw-link "$raw_link" >"$TMP_DIR/prune.json"

[ -d "$good_runtime" ] || fail 'protected verified runtime was pruned'
[ -d "$running_runtime" ] || fail 'protected running runtime was pruned'
[ -d "$good_raw" ] || fail 'protected raw store was pruned'

rm -rf "$current_link" "$raw_link"
cp -a "$good_runtime" "$current_link"
printf 'physical drift\n' >>"$current_link/codex"
chmod 755 "$current_link/codex"
cp -a "$good_raw" "$raw_link"
printf 'physical raw drift\n' >>"$raw_link/vendor/aarch64-unknown-linux-musl/bin/codex"
chmod 755 "$raw_link/vendor/aarch64-unknown-linux-musl/bin/codex"
runtime_candidate="$TMP_DIR/runtime-candidate"
raw_candidate="$TMP_DIR/raw-candidate"
cp -a "$good_runtime" "$runtime_candidate"
cp -a "$good_raw" "$raw_candidate"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B -m codex_termux.cli activation-commit \
    --candidate-runtime "$runtime_candidate" \
    --candidate-raw "$raw_candidate" \
    --runtime-target "$runtime_store/physical-migration-runtime" \
    --raw-target "$raw_store/physical-migration-raw" \
    --version 0.142.0-linux-arm64 \
    --raw-sha256 "$raw_sha" \
    --runtime-sha256 "$runtime_sha" \
    --package-spec @openai/codex@0.142.0-linux-arm64 \
    --cleanup-runtime-source \
    --cleanup-raw-source \
    --current-link "$current_link" \
    --verified-link "$verified_link" \
    --raw-link "$raw_link" \
    --state-file "$state_file" \
    --registry-file "$registry_file" \
    --runtime-store-dir "$runtime_store" \
    --raw-store-dir "$raw_store" \
    --wrapper-version 20260623-1 \
    --wrapper-commit testcommit \
    --updated-at 2026-01-02T00:00:00+00:00 \
    --shell-bin "$shell_bin" \
    --shell-lib "$ROOT_DIR/lib/codex-termux.sh" \
    --home "$TMP_DIR/home" \
    --prefix "$TMP_DIR/prefix" \
    --manager-dir "$manager_dir" \
    --runtime-builder "$manager_dir/build-runtime.py" \
    --resolv-conf "$resolv_conf" \
    --cert-file "$cert_file" \
    --cert-dir "$cert_dir" \
    --patch-policy termux-fd-remap-v1 >/dev/null

[ -L "$current_link" ] || fail 'physical current was not migrated to a symlink'
[ "$(readlink "$current_link")" = "$runtime_store/physical-migration-runtime" ] || fail 'physical current migrated to wrong runtime'
[ -L "$raw_link" ] || fail 'physical raw was not migrated to a symlink'
[ "$(readlink "$raw_link")" = "$raw_store/physical-migration-raw" ] || fail 'physical raw migrated to wrong raw'
[ "$(cat "$runtime_store/physical-migration-runtime/upstream/upstream-only.txt")" = 'upstream-only' ] || fail 'upstream tree entry was not preserved in the immutable store'

printf 'store-rollback: ok\n'
