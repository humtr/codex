#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'termux rebuild smoke: FAIL: %s\n' "$*" >&2
    exit 1
}

case "${PREFIX:-}" in
    /data/data/com.termux/files/usr) ;;
    *) fail 'this test must run in a real Termux environment' ;;
esac

printf '== runtime-artifact ==\n'
bash "$ROOT_DIR/tests/runtime-build.sh"

printf '== installed-facades ==\n'
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
manager="$tmp_dir/manager"
mkdir -p "$manager/source/libexec"
cp "$ROOT_DIR/tools/build-runtime.py" "$manager/build-runtime.py"
cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager/bwrap-termux-compat.py"
cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager/rg-termux-shim.sh"
cp -R "$ROOT_DIR/libexec/." "$manager/source/libexec/"
chmod 755 "$manager"/*.py "$manager"/*.sh "$manager/source/libexec"/*
PYTHONDONTWRITEBYTECODE=1 python3 -B "$manager/bwrap-termux-compat.py" -- /definitely/missing/runtime-artifact-smoke 2>"$tmp_dir/bwrap.err" && fail 'installed bwrap facade accepted missing executable'
grep -F 'failed to exec' "$tmp_dir/bwrap.err" >/dev/null || fail 'installed bwrap facade did not reach implementation'
printf 'termux rebuild smoke: ok\n'
