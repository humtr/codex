#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'wrapper-archive-safety: FAIL: %s\n' "$*" >&2
    exit 1
}

make_tarball() {
    local out="$1" kind="$2"
    python3 -B - "$out" "$kind" <<'PYTHON'
import io
import sys
import tarfile

out, kind = sys.argv[1:3]
with tarfile.open(out, "w:gz") as tf:
    if kind == "safe":
        data = b"ok\n"
        info = tarfile.TarInfo("repo/install.sh")
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    elif kind == "traversal":
        data = b"bad\n"
        info = tarfile.TarInfo("repo/../escape.txt")
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    elif kind == "absolute":
        data = b"bad\n"
        info = tarfile.TarInfo("/tmp/escape.txt")
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    elif kind == "symlink":
        info = tarfile.TarInfo("repo/link")
        info.type = tarfile.SYMTYPE
        info.linkname = "/etc/passwd"
        info.size = 0
        tf.addfile(info)
    elif kind == "hardlink":
        info = tarfile.TarInfo("repo/hard")
        info.type = tarfile.LNKTYPE
        info.linkname = "repo/install.sh"
        info.size = 0
        tf.addfile(info)
    elif kind == "special":
        info = tarfile.TarInfo("repo/device")
        info.type = tarfile.CHRTYPE
        info.devmajor = 1
        info.devminor = 3
        info.size = 0
        tf.addfile(info)
    else:
        raise SystemExit(f"unknown kind: {kind}")
PYTHON
}

grep -F 'bootstrap_validate_tarball_safe "$archive"' "$ROOT_DIR/install.sh" >/dev/null \
    || fail 'bootstrap install does not validate wrapper archive before extraction'
grep -F 'codex_termux_cmd validate-tarball --path "$archive"' "$ROOT_DIR/bin/install-runtime.sh" >/dev/null \
    || fail 'install-runtime release source path does not validate wrapper archive before extraction'

# shellcheck disable=SC1090
. "$ROOT_DIR/install.sh"

safe="$TMP_DIR/safe.tgz"
make_tarball "$safe" safe
bootstrap_validate_tarball_safe "$safe" || fail 'safe wrapper archive rejected'

for kind in traversal absolute symlink hardlink special; do
    tgz="$TMP_DIR/$kind.tgz"
    make_tarball "$tgz" "$kind"
    if bootstrap_validate_tarball_safe "$tgz" 2>/dev/null; then
        fail "unsafe wrapper archive accepted: $kind"
    fi
done

printf 'wrapper-archive-safety: ok\n'
