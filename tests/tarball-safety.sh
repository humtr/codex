#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'tarball-safety: FAIL: %s\n' "$*" >&2
    exit 1
}

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux-lib.sh"

safe_tgz="$fixture_root/safe.tgz"
python3 - "$safe_tgz" <<'PY'
import io
import tarfile
import sys

path = sys.argv[1]
with tarfile.open(path, "w:gz") as tf:
    data = b"ok\n"
    info = tarfile.TarInfo("package/file.txt")
    info.size = len(data)
    tf.addfile(info, io.BytesIO(data))
PY

unsafe_traversal="$fixture_root/traversal.tgz"
python3 - "$unsafe_traversal" <<'PY'
import io
import tarfile
import sys

path = sys.argv[1]
with tarfile.open(path, "w:gz") as tf:
    data = b"bad\n"
    info = tarfile.TarInfo("../escape.txt")
    info.size = len(data)
    tf.addfile(info, io.BytesIO(data))
PY

unsafe_symlink="$fixture_root/symlink.tgz"
python3 - "$unsafe_symlink" <<'PY'
import tarfile
import sys

path = sys.argv[1]
with tarfile.open(path, "w:gz") as tf:
    info = tarfile.TarInfo("package/link")
    info.type = tarfile.SYMTYPE
    info.linkname = "/etc/passwd"
    tf.addfile(info)
PY

unsafe_hardlink="$fixture_root/hardlink.tgz"
python3 - "$unsafe_hardlink" <<'PY'
import tarfile
import sys

path = sys.argv[1]
with tarfile.open(path, "w:gz") as tf:
    info = tarfile.TarInfo("package/hard")
    info.type = tarfile.LNKTYPE
    info.linkname = "package/file.txt"
    tf.addfile(info)
PY

unsafe_special="$fixture_root/special.tgz"
python3 - "$unsafe_special" <<'PY'
import tarfile
import sys

path = sys.argv[1]
with tarfile.open(path, "w:gz") as tf:
    info = tarfile.TarInfo("package/device")
    info.type = tarfile.CHRTYPE
    info.devmajor = 1
    info.devminor = 3
    tf.addfile(info)
PY

codex_validate_tarball_safe "$safe_tgz" >/dev/null || fail "safe tarball rejected"
for tgz in "$unsafe_traversal" "$unsafe_symlink" "$unsafe_hardlink" "$unsafe_special"; do
    if codex_validate_tarball_safe "$tgz" >/dev/null 2>&1; then
        fail "unsafe tarball accepted: $tgz"
    fi
done

printf 'tarball-safety: ok\n'
