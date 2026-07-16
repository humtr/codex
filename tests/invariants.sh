#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-invariants.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'invariants: FAIL: %s\n' "$*" >&2
    exit 1
}

cd "$ROOT_DIR"

for term in \
    "codex""_native" \
    "CODEX""_NATIVE" \
    "codex/""native" \
    "codex"" ""native" \
    "native"".lock" \
    "CODEX_TERMUX""_RESOLVER_FD" \
    "CODEX_TERMUX""_SHARED_PLUGINS_DIR" \
    "codex_profile""_share_plugins"
do
    if grep -RIn \
        --exclude-dir=.git \
        --exclude-dir=__pycache__ \
        --exclude-dir=out \
        --exclude='*.pyc' \
        -- "$term" . >"$TMP_DIR/forbidden" 2>/dev/null
    then
        cat "$TMP_DIR/forbidden" >&2
        fail "forbidden legacy contract term remains: $term"
    fi
done

grep -E '^CODEX_TERMUX_WRAPPER_VERSION=[0-9]{6}-[0-9]+$' config/wrapper-version.env >/dev/null \
    || fail 'wrapper version mismatch'
grep -Fx 'CODEX_TERMUX_WRAPPER_CHANNEL=termux' config/wrapper-version.env >/dev/null \
    || fail 'wrapper channel mismatch'
grep -Fx 'CODEX_TERMUX_WRAPPER_REPO=humtr/codex' config/wrapper-version.env >/dev/null \
    || fail 'wrapper repo mismatch'

for path in \
    shell/loader.sh \
    shell/state.sh \
    shell/exec.sh \
    shell/dispatch.sh \
    src/wrapper/cli.py \
    src/wrapper/source.py \
    src/wrapper/prune.py \
    src/wrapper/notification/model.py \
    src/wrapper/notification/service.py \
    libexec/notify \
    libexec/build-runtime.py \
    libexec/bwrap-termux-compat.py \
    libexec/rg-termux-shim.sh \
    native/codex-launcher.c
 do
    [ -e "$path" ] || fail "role-oriented path missing: $path"
done

[ "$(find tools/codex_termux -maxdepth 1 -type f | wc -l | tr -d ' ')" = 3 ] \
    || fail 'legacy Python package contains implementation files'
for path in tools/codex_termux/__init__.py tools/codex_termux/cli.py tools/codex_termux/notify.py; do
    [ -f "$path" ] || fail "legacy Python facade missing: $path"
done

for path in lib/codex-termux/*.sh; do
    grep -F 'missing wrapper shell domain:' "$path" >/dev/null \
        || fail "legacy shell path contains implementation: $path"
done

grep -F 'wrapper_cmd()' shell/state.sh >/dev/null \
    || fail 'role-oriented helper command bridge missing'
grep -F 'codex_termux_cmd()' shell/state.sh >/dev/null \
    || fail 'legacy helper command alias missing'
grep -F '33<"$CODEX_TERMUX_RESOLV_CONF"' shell/exec.sh >/dev/null \
    || fail 'runtime fd33 launcher contract missing'
grep -F '34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"' shell/exec.sh >/dev/null \
    || fail 'runtime fd34 launcher contract missing'
grep -F 'PATCH_POLICY = "termux-fd-remap-v1"' libexec/build-runtime.py >/dev/null \
    || fail 'builder patch policy changed'
grep -F 'b"/etc/resolv.conf": b"/proc/self/fd/33"' libexec/build-runtime.py >/dev/null \
    || fail 'builder resolver target changed'
grep -F 'b"/etc/codex/config.toml": b"/dev/fd/34/config.toml"' libexec/build-runtime.py >/dev/null \
    || fail 'builder system config target changed'
grep -F 'prepare_support_install' bin/install-runtime.sh >/dev/null \
    || fail 'transactional support prepare missing'
grep -F 'rollback_support_install' bin/install-runtime.sh >/dev/null \
    || fail 'transactional support rollback missing'
grep -F 'commit_support_install' bin/install-runtime.sh >/dev/null \
    || fail 'transactional support commit missing'
grep -F 'install upstream [VERSION]' bin/install-runtime.sh >/dev/null \
    || fail 'install upstream help surface missing'
grep -F 'install rebuild' bin/install-runtime.sh >/dev/null \
    || fail 'install rebuild help surface missing'
grep -F 'codex_runtime_install_upstream()' shell/build.sh >/dev/null \
    || fail 'runtime upstream install helper missing'
grep -F 'codex_runtime_install_cached()' shell/build.sh >/dev/null \
    || fail 'runtime cached install helper missing'

if grep -R -E 'shell[[:space:]]*=[[:space:]]*True|os\.system\(' src/wrapper/notification >/dev/null; then
    fail 'notification subsystem allows shell execution'
fi

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=src python3 -B -m wrapper.cli validate --root "$ROOT_DIR" >/dev/null
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=src:tools python3 -B -m codex_termux.cli --help >/dev/null
PYTHONDONTWRITEBYTECODE=1 python3 -B libexec/notify self-test >/dev/null

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=src python3 -B - <<'PYTHON'
from pathlib import Path

root = Path("src/wrapper")
for path in sorted(root.rglob("*.py")):
    compile(path.read_text(encoding="utf-8"), str(path), "exec")
PYTHON

bytecode_found="$(find . \( -type d -name '__pycache__' -o -type f -name '*.pyc' \) -print -quit 2>/dev/null || true)"
[ -z "$bytecode_found" ] || fail "bytecode artifact found: $bytecode_found"

if grep -R -F 'codex rebuild' README.md >/dev/null; then
    fail 'stale public codex rebuild command remains in README'
fi

printf 'invariants: ok\n'
