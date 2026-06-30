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
copy_path tools/codex-launcher.c
copy_path tools/codex-turn-notify.sh
copy_path tools/codex_termux
copy_path config
copy_path codex-wrapper.manifest.json

find "$PACKAGE_ROOT" \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -exec rm -rf {} +

for forbidden in tests .github docs .agents .git .gitignore; do
    if [ -e "$PACKAGE_ROOT/$forbidden" ]; then
        printf 'package-release: forbidden release entry: %s\n' "$forbidden" >&2
        exit 1
    fi
done
for forbidden in tools/install-git-hooks.sh tools/update-wrapper-version.sh; do
    if [ -e "$PACKAGE_ROOT/$forbidden" ]; then
        printf 'package-release: forbidden development tool in release: %s\n' "$forbidden" >&2
        exit 1
    fi
done

removed_terms=(
    "codex""_native"
    "CODEX""_NATIVE"
    "codex/""native"
    "codex"" native"
    "native"".lock"
    "CODEX_TERMUX""_RESOLVER_FD"
    "CODEX_TERMUX""_SHARED_PLUGINS_DIR"
    "codex_profile""_share_plugins"
)
for term in "${removed_terms[@]}"; do
    if grep -RIn -e "$term" "$PACKAGE_ROOT" >/dev/null; then
        printf 'package-release: removed legacy contract remains in release package: %s\n' "$term" >&2
        exit 1
    fi
done

mkdir -p "$(dirname "$OUT")"
PYTHONDONTWRITEBYTECODE=1 python3 -B - "$PACKAGE_ROOT" "$OUT" <<'PYTHON'
from __future__ import annotations

import os
import sys
import zipfile
from pathlib import Path

root = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
if out.exists():
    out.unlink()
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in sorted(root.rglob("*")):
        if path.is_dir():
            continue
        rel = path.relative_to(root.parent)
        info = zipfile.ZipInfo(str(rel).replace(os.sep, "/"))
        info.external_attr = (0o755 if os.access(path, os.X_OK) else 0o644) << 16
        with path.open("rb") as handle:
            zf.writestr(info, handle.read())
PYTHON

printf '%s\n' "$OUT"
