#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
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

python3 - "$out" <<'PYTHON' || exit 1
import sys
import zipfile
from pathlib import PurePosixPath

zip_path = sys.argv[1]
forbidden_roots = {"tests", ".github", "docs", ".agents", ".git"}
forbidden_exact = {"GOAL.md", ".gitignore", "tools/install-git-hooks.sh", "tools/update-wrapper-version.sh"}
required = {
    "README.md",
    "install.sh",
    "bin/install-local.sh",
    "bin/install-runtime.sh",
    "lib/codex-termux.sh",
    "lib/codex-termux/dispatch.sh",
    "lib/codex-termux/state.sh",
    "lib/codex-termux/profile.sh",
    "lib/codex-termux/session.sh",
    "lib/codex-termux/runtime.sh",
    "lib/codex-termux/notify.sh",
    "lib/codex-termux/doctor.sh",
    "codex-wrapper.manifest.json",
    "tools/build-runtime.py",
    "tools/bwrap-termux-compat.py",
    "tools/rg-termux-shim.sh",
    "tools/codex-launcher.c",
    "tools/codex-turn-notify.sh",
    "tools/codex_termux/cli.py",
    "tools/codex_termux/cli_activation.py",
    "tools/codex_termux/cli_artifacts.py",
    "tools/codex_termux/cli_doctor.py",
    "tools/codex_termux/cli_notify.py",
    "tools/codex_termux/cli_product.py",
    "tools/codex_termux/cli_profile.py",
    "tools/codex_termux/cli_session.py",
    "tools/codex_termux/cli_ui.py",
    "tools/codex_termux/ui.py",
    "config/wrapper-version.env",
}
with zipfile.ZipFile(zip_path) as zf:
    stripped = set()
    for name in zf.namelist():
        parts = PurePosixPath(name).parts
        if parts and parts[0].startswith("codex-termux-"):
            rel = PurePosixPath(*parts[1:]) if len(parts) > 1 else PurePosixPath("")
            stripped.add(str(rel))
        else:
            stripped.add(name)
    missing = sorted(required - stripped)
    if missing:
        raise SystemExit(f"missing release entries: {missing}")
    for rel in stripped:
        if not rel:
            continue
        first = rel.split("/", 1)[0]
        if first in forbidden_roots or rel in forbidden_exact:
            raise SystemExit(f"forbidden release entry: {rel}")
PYTHON

printf 'release-package: ok\n'
