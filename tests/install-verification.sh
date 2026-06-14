#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'install-verification: FAIL: %s\n' "$*" >&2
    exit 1
}

run_case() {
    local version_exit="$1" expect_success="$2" fixture_root prefix status
    fixture_root="$(mktemp -d)"
    prefix="$fixture_root/prefix"
    mkdir -p "$prefix/bin"
    set +e
    (
        set -euo pipefail
        export PREFIX="$prefix"
        source "$ROOT_DIR/install.sh"
        need_termux() { :; }
        install_dependencies() { :; }
        bash() {
            case "$1" in
                "$ROOT_DIR/bin/install-runtime.sh")
                    case "$2" in
                        setup)
                            cat >"$PREFIX/bin/codex" <<EOF
#!/bin/sh
if [ "\${1:-}" = version ]; then
    exit $version_exit
fi
exit 0
EOF
                            chmod 755 "$PREFIX/bin/codex"
                            return 0
                            ;;
                        doctor)
                            return 0
                            ;;
                    esac
                    ;;
            esac
            command bash "$@"
        }
        set +e
        main
        status=$?
        set -e
        exit "$status"
    )
    status=$?
    set -e
    rm -rf "$fixture_root"
    if [ "$expect_success" = "yes" ]; then
        [ "$status" -eq 0 ] || fail "expected success but got $status"
    else
        [ "$status" -ne 0 ] || fail "expected failure but main succeeded"
    fi
}

run_case 0 yes
run_case 1 no

printf 'install-verification: ok\n'
