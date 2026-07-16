#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-release-test.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'release-package: FAIL: %s\n' "$*" >&2
    exit 1
}

out="$TMP_DIR/codex-release.zip"
actual="$(TMPDIR="$TMP_DIR" bash "$ROOT_DIR/tools/package-release.sh" "$out")" \
    || fail 'package-release script failed'
[ "$actual" = "$out" ] || fail "package-release printed unexpected output: $actual"
[ -s "$out" ] || fail 'release zip was not created'

python3 -B - "$out" <<'PYTHON'
import sys
import zipfile
from pathlib import PurePosixPath

zip_path = sys.argv[1]
forbidden_roots = {"tests", ".github", "docs", ".agents", ".git", "dist"}
forbidden_exact = {
    "GOAL.md",
    ".gitignore",
    "tools/install-git-hooks.sh",
    "tools/update-wrapper-version.sh",
    "tools/golden_capture.py",
}
required = {
    "README.md",
    "install.sh",
    "bin/install-local.sh",
    "bin/install-runtime.sh",
    "lib/codex-termux.sh",
    "lib/codex-termux/exec.sh",
    "shell/loader.sh",
    "shell/state.sh",
    "shell/exec.sh",
    "shell/dispatch.sh",
    "src/wrapper/__init__.py",
    "src/wrapper/cli.py",
    "src/wrapper/source.py",
    "src/wrapper/support_layout.py",
    "src/wrapper/prune.py",
    "src/wrapper/notification/model.py",
    "src/wrapper/notification/service.py",
    "src/wrapper/notification/provider.py",
    "src/codex_termux/__init__.py",
    "libexec/notify",
    "libexec/build-runtime.py",
    "libexec/bwrap-termux-compat.py",
    "libexec/rg-termux-shim.sh",
    "native/codex-launcher.c",
    "tools/build-runtime.py",
    "tools/bwrap-termux-compat.py",
    "tools/rg-termux-shim.sh",
    "tools/termux-notify.sh",
    "tools/codex-launcher.c",
    "tools/codex-turn-notify.sh",
    "tools/codex_termux/__init__.py",
    "tools/codex_termux/cli.py",
    "tools/codex_termux/notify.py",
    "config/wrapper-version.env",
    "config/layout-contracts.json",
    "codex-wrapper.manifest.json",
}
with zipfile.ZipFile(zip_path) as zf:
    stripped = set()
    executable = set()
    for info in zf.infolist():
        parts = PurePosixPath(info.filename).parts
        if parts and parts[0].startswith("codex-termux-"):
            rel = PurePosixPath(*parts[1:]) if len(parts) > 1 else PurePosixPath("")
            name = str(rel)
        else:
            name = info.filename
        stripped.add(name)
        if (info.external_attr >> 16) & 0o111:
            executable.add(name)
    missing = sorted(required - stripped)
    if missing:
        raise SystemExit(f"missing release entries: {missing}")
    for rel in stripped:
        if not rel:
            continue
        first = rel.split("/", 1)[0]
        if first in forbidden_roots or rel in forbidden_exact:
            raise SystemExit(f"forbidden release entry: {rel}")
        if "__pycache__" in rel.split("/") or rel.endswith(".pyc"):
            raise SystemExit(f"bytecode release entry: {rel}")
    for rel in (
        "libexec/notify",
        "libexec/build-runtime.py",
        "libexec/bwrap-termux-compat.py",
        "libexec/rg-termux-shim.sh",
    ):
        if rel not in executable:
            raise SystemExit(f"release entry is not executable: {rel}")
PYTHON

printf 'release-package: ok\n'
