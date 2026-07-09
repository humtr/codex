#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^CODEX_TERMUX_WRAPPER_VERSION=//p' "$ROOT_DIR/config/wrapper-version.env" | head -n 1)"
VERSION="${VERSION:-unknown}"
OUT="${1:-$ROOT_DIR/dist/codex-termux-$VERSION.zip}"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
STAGE="$(mktemp -d "$TMP_PARENT/codex-release.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
PACKAGE_ROOT="$STAGE/codex-termux-$VERSION"

copy_path() {
    local rel="$1"
    mkdir -p "$PACKAGE_ROOT/$(dirname "$rel")"
    if [ -d "$ROOT_DIR/$rel" ]; then
        cp -R "$ROOT_DIR/$rel" "$PACKAGE_ROOT/$rel"
    else
        cp "$ROOT_DIR/$rel" "$PACKAGE_ROOT/$rel"
    fi
}

mkdir -p "$PACKAGE_ROOT"
copy_path README.md
copy_path install.sh
copy_path bin
copy_path lib
mkdir -p "$PACKAGE_ROOT/tools"
copy_path tools/build-runtime.py
copy_path tools/bwrap-termux-compat.py
copy_path tools/rg-termux-shim.sh
copy_path tools/termux-notify.sh
copy_path tools/codex-launcher.c
copy_path tools/codex-turn-notify.sh
copy_path tools/codex_termux
copy_path config
copy_path codex-wrapper.manifest.json

find "$PACKAGE_ROOT" \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -exec rm -rf {} +

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B -m codex_termux.cli \
    release-package --package-root "$PACKAGE_ROOT" --out "$OUT"
