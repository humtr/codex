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
for rel in \
    README.md \
    install.sh \
    bin \
    lib \
    shell \
    src \
    libexec \
    native \
    config \
    codex-wrapper.manifest.json
do
    copy_path "$rel"
done

mkdir -p "$PACKAGE_ROOT/tools"
for rel in \
    tools/build-runtime.py \
    tools/bwrap-termux-compat.py \
    tools/rg-termux-shim.sh \
    tools/termux-notify.sh \
    tools/codex-launcher.c \
    tools/codex-turn-notify.sh \
    tools/codex_termux
do
    copy_path "$rel"
done

find "$PACKAGE_ROOT" \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -exec rm -rf {} +
chmod 755 \
    "$PACKAGE_ROOT/libexec/notify" \
    "$PACKAGE_ROOT/libexec/build-runtime.py" \
    "$PACKAGE_ROOT/libexec/bwrap-termux-compat.py" \
    "$PACKAGE_ROOT/libexec/bwrap-compat.py" \
    "$PACKAGE_ROOT/libexec/rg-termux-shim.sh" \
    "$PACKAGE_ROOT/libexec/rg-shim.sh"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B -m wrapper.cli \
    release-package --package-root "$PACKAGE_ROOT" --out "$OUT"
