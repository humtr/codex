#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'package-safety: FAIL: %s\n' "$*" >&2
    exit 1
}

make_tarball() {
    local out="$1" kind="$2"
    PYTHONDONTWRITEBYTECODE=1 python3 -B - "$out" "$kind" <<'PYTHON'
import io
import sys
import tarfile
from pathlib import PurePosixPath

out, kind = sys.argv[1:3]
with tarfile.open(out, "w:gz") as tf:
    if kind == "safe":
        data = b"ok\n"
        info = tarfile.TarInfo("package/file.txt")
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
        directory = tarfile.TarInfo("package/dir")
        directory.type = tarfile.DIRTYPE
        tf.addfile(directory)
    elif kind == "traversal":
        data = b"bad\n"
        info = tarfile.TarInfo("../escape.txt")
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    elif kind == "absolute":
        data = b"bad\n"
        info = tarfile.TarInfo("/tmp/escape.txt")
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    elif kind == "symlink":
        info = tarfile.TarInfo("package/link")
        info.type = tarfile.SYMTYPE
        info.linkname = "/etc/passwd"
        tf.addfile(info)
    elif kind == "hardlink":
        info = tarfile.TarInfo("package/hard")
        info.type = tarfile.LNKTYPE
        info.linkname = "package/file.txt"
        tf.addfile(info)
    elif kind == "special":
        info = tarfile.TarInfo("package/device")
        info.type = tarfile.CHRTYPE
        info.devmajor = 1
        info.devminor = 3
        tf.addfile(info)
    else:
        raise SystemExit(f"unknown kind: {kind}")
PYTHON
}

run_validator() {
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B -m codex_termux.cli \
        validate-tarball --path "$1" >/dev/null
}

safe="$TMP_DIR/safe.tgz"
make_tarball "$safe" safe
run_validator "$safe" || fail 'safe tarball rejected'

for kind in traversal absolute symlink hardlink special; do
    tgz="$TMP_DIR/$kind.tgz"
    make_tarball "$tgz" "$kind"
    if run_validator "$tgz" 2>/dev/null; then
        fail "unsafe tarball accepted: $kind"
    fi
done

printf 'package-safety: ok\n'
