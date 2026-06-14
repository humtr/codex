#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CODEX_NATIVE_REQUIRED_PACKAGES="${CODEX_NATIVE_REQUIRED_PACKAGES:-bash curl nodejs python tar coreutils ca-certificates}"
CODEX_NATIVE_LOG_PREFIX="codex setup"
export DEBIAN_FRONTEND="${DEBIAN_FRONTEND:-noninteractive}"

say() {
    printf '%s: %s\n' "$CODEX_NATIVE_LOG_PREFIX" "$*" >&2
}

fail() {
    printf '%s: ERROR: %s\n' "$CODEX_NATIVE_LOG_PREFIX" "$*" >&2
    exit 1
}

need_termux() {
    [ -n "${PREFIX:-}" ] || fail 'PREFIX is not set. Run this inside Termux.'
    [ -x "$PREFIX/bin/pkg" ] || fail 'Termux pkg command not found.'
}

install_dependencies() {
    local missing=() package
    for package in $CODEX_NATIVE_REQUIRED_PACKAGES; do
        if ! dpkg -s "$package" >/dev/null 2>&1; then
            missing+=("$package")
        fi
    done
    if [ "${#missing[@]}" -eq 0 ]; then
        return 0
    fi
    say "installing dependencies: ${missing[*]}"
    apt-get install -y \
        -o Dpkg::Options::=--force-confdef \
        -o Dpkg::Options::=--force-confold \
        "${missing[@]}" || fail "dependency install failed: ${missing[*]}"
}

main() {
    need_termux
    say 'checking dependencies'
    install_dependencies
    say 'installing managed runtime'
    bash "$ROOT_DIR/bin/install-runtime.sh" setup "$@"
    say 'verifying public launcher'
    "$PREFIX/bin/codex" version >/dev/null || fail 'public Codex launcher version check failed'
    say 'verifying wrapper diagnostics'
    bash "$ROOT_DIR/bin/install-runtime.sh" doctor --json >/dev/null \
        || fail 'wrapper doctor verification failed'
    say 'ok'
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    main "$@"
fi
