#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="$ROOT_DIR/scratch"

fail() {
    printf 'immutable-store: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"
FIXTURE_ROOT="$(mktemp -d "$FIXTURE_PARENT/immutable-store-test.XXXXXX")"
trap 'rm -rf "$FIXTURE_ROOT"' EXIT

manager_dir="$FIXTURE_ROOT/native/manager"
runtime_src="$FIXTURE_ROOT/runtime"
raw_src="$FIXTURE_ROOT/raw"
raw_vendor="$raw_src/vendor/aarch64-unknown-linux-musl"
mkdir -p "$manager_dir" "$runtime_src/codex-resources" "$runtime_src/codex-path" "$raw_vendor/bin"

cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
chmod 755 "$manager_dir/build-runtime.py" "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"

printf '#!/bin/sh\nexit 0\n' >"$runtime_src/codex"
printf 'resource-a\n' >"$runtime_src/codex-resources/data"
printf 'path-a\n' >"$runtime_src/codex-path/data"
printf '{"name":"a"}\n' >"$runtime_src/codex-package.json"
printf '{"manifest":"a"}\n' >"$runtime_src/runtime-build.json"
printf 'raw-a\n' >"$raw_vendor/bin/codex"
printf 'raw-resource-a\n' >"$raw_vendor/resource"
chmod 755 "$runtime_src/codex" "$raw_vendor/bin/codex"

export CODEX_NATIVE_HOME="$FIXTURE_ROOT/home"
export CODEX_NATIVE_NATIVE_ROOT="$FIXTURE_ROOT/native"
export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"
export CODEX_NATIVE_STORE_DIR="$FIXTURE_ROOT/native/store"

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

runtime_sha="$(codex_sha256 "$runtime_src/codex")"
raw_sha="$(codex_sha256 "$raw_vendor/bin/codex")"
runtime_path="$(codex_store_runtime_payload test "$runtime_sha" "$runtime_src")" \
    || fail "initial runtime publish failed"
raw_path="$(codex_store_raw_payload test "$raw_sha" "$raw_src")" \
    || fail "initial raw publish failed"
runtime_digest="$(codex_tree_digest "$runtime_path")"
raw_digest="$(codex_tree_digest "$raw_path")"

codex_store_runtime_payload test "$runtime_sha" "$runtime_src" >/dev/null \
    || fail "identical runtime reuse failed"
codex_store_raw_payload test "$raw_sha" "$raw_src" >/dev/null \
    || fail "identical raw reuse failed"

runtime_variant="$FIXTURE_ROOT/runtime-variant"
raw_variant="$FIXTURE_ROOT/raw-variant"
cp -R "$runtime_src" "$runtime_variant"
cp -R "$raw_src" "$raw_variant"
printf '{"name":"variant"}\n' >"$runtime_variant/codex-package.json"
printf 'raw-b\n' >"$raw_variant/resource"
variant_runtime_path="$(codex_store_runtime_payload test "$runtime_sha" "$runtime_variant")" \
    || fail "runtime tree-digest variant publish failed"
variant_raw_path="$(codex_store_raw_payload test "$raw_sha" "$raw_variant")" \
    || fail "raw tree-digest variant publish failed"
[ "$variant_runtime_path" != "$runtime_path" ] \
    || fail "runtime tree-digest variant reused the original store path"
[ "$variant_raw_path" != "$raw_path" ] \
    || fail "raw tree-digest variant reused the original store path"

printf '{"name":"collision"}\n' >"$runtime_src/codex-package.json"
modified_runtime_path="$(codex_store_runtime_payload test "$runtime_sha" "$runtime_src")" \
    || fail "modified runtime publish failed"
[ "$modified_runtime_path" != "$runtime_path" ] \
    || fail "modified runtime reused the original store path"
[ "$(codex_tree_digest "$runtime_path")" = "$runtime_digest" ] \
    || fail "modified runtime publish modified existing artifact"

printf 'raw-resource-collision\n' >"$raw_vendor/resource"
modified_raw_path="$(codex_store_raw_payload test "$raw_sha" "$raw_src")" \
    || fail "modified raw publish failed"
[ "$modified_raw_path" != "$raw_path" ] \
    || fail "modified raw reused the original store path"
[ "$(codex_tree_digest "$raw_path")" = "$raw_digest" ] \
    || fail "modified raw publish modified existing artifact"

printf '{"name":"a"}\n' >"$runtime_src/codex-package.json"
target_mode="$(stat -c '%a' "$runtime_path")"
if [ "$target_mode" = "700" ]; then
    chmod 755 "$runtime_src"
else
    chmod 700 "$runtime_src"
fi
if codex_publish_immutable_tree "$runtime_src" "$runtime_path" >/dev/null 2>&1; then
    fail "runtime store root permission collision was accepted"
fi
[ "$(codex_tree_digest "$runtime_path")" = "$runtime_digest" ] \
    || fail "root permission collision modified existing artifact"

symlink_source="$FIXTURE_ROOT/symlink-source"
symlink_target="$FIXTURE_ROOT/symlink-target"
symlink_destination="$FIXTURE_ROOT/symlink-destination"
mkdir -p "$symlink_source" "$symlink_destination"
printf 'source\n' >"$symlink_source/data"
ln -s "$symlink_destination" "$symlink_target"
if codex_publish_immutable_tree "$symlink_source" "$symlink_target" >/dev/null 2>&1; then
    fail "symlink target collision was accepted"
fi
[ -L "$symlink_target" ] || fail "symlink target collision replaced target"
[ -d "$symlink_source" ] || fail "symlink target collision consumed source"

special_source="$FIXTURE_ROOT/special-source"
mkdir -p "$special_source"
mkfifo "$special_source/fifo"
if codex_tree_digest "$special_source" >/dev/null 2>&1; then
    fail "special file tree digest was accepted"
fi

same_target="$FIXTURE_ROOT/concurrent-same-target"
same_source_one="$FIXTURE_ROOT/concurrent-same-one"
same_source_two="$FIXTURE_ROOT/concurrent-same-two"
mkdir -p "$same_source_one" "$same_source_two"
printf 'same\n' >"$same_source_one/data"
cp -R "$same_source_one/." "$same_source_two/"
set +e
codex_publish_immutable_tree "$same_source_one" "$same_target" >/dev/null 2>&1 &
same_pid_one=$!
codex_publish_immutable_tree "$same_source_two" "$same_target" >/dev/null 2>&1 &
same_pid_two=$!
wait "$same_pid_one"
same_rc_one=$?
wait "$same_pid_two"
same_rc_two=$?
set -e
[ "$same_rc_one" -eq 0 ] && [ "$same_rc_two" -eq 0 ] ||
    fail "identical concurrent publish did not succeed twice"
[ ! -e "$same_source_one" ] && [ ! -e "$same_source_two" ] ||
    fail "identical concurrent publish did not consume both sources"

different_target="$FIXTURE_ROOT/concurrent-different-target"
different_source_one="$FIXTURE_ROOT/concurrent-different-one"
different_source_two="$FIXTURE_ROOT/concurrent-different-two"
mkdir -p "$different_source_one" "$different_source_two"
printf 'one\n' >"$different_source_one/data"
printf 'two\n' >"$different_source_two/data"
set +e
codex_publish_immutable_tree "$different_source_one" "$different_target" >/dev/null 2>&1 &
different_pid_one=$!
codex_publish_immutable_tree "$different_source_two" "$different_target" >/dev/null 2>&1 &
different_pid_two=$!
wait "$different_pid_one"
different_rc_one=$?
wait "$different_pid_two"
different_rc_two=$?
set -e
[ $((different_rc_one + different_rc_two)) -eq 1 ] ||
    fail "different concurrent publish did not produce one collision"
[ -d "$different_target" ] || fail "different concurrent publish created no target"

printf 'immutable-store: ok\n'
