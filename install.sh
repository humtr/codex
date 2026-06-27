#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CODEX_TERMUX_REQUIRED_PACKAGES="${CODEX_TERMUX_REQUIRED_PACKAGES:-bash curl nodejs python tar coreutils ca-certificates}"
CODEX_TERMUX_INSTALL_VERSION_OUTPUT="${CODEX_TERMUX_INSTALL_VERSION_OUTPUT:-1}"
CODEX_TERMUX_INSTALL_OK_OUTPUT="${CODEX_TERMUX_INSTALL_OK_OUTPUT:-0}"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="${CODEX_TERMUX_WRAPPER_SOURCE_CONFIG:-$HOME/.config/codex-termux/wrapper-source.env}"
export DEBIAN_FRONTEND="${DEBIAN_FRONTEND:-noninteractive}"
CODEX_TERMUX_INSTALL_STATUS_ACTIVE=0
CODEX_TERMUX_INSTALL_LOG=""

install_tmp_dir() {
    local candidate
    for candidate in "${TMPDIR:-}" "${PREFIX:-/data/data/com.termux/files/usr}/tmp"; do
        [ -n "$candidate" ] || continue
        case "$candidate" in
            /tmp) continue ;;
            /*) ;;
            *) continue ;;
        esac
        if mkdir -p "$candidate" 2>/dev/null && [ -d "$candidate" ] && [ -w "$candidate" ]; then
            printf '%s\n' "${candidate%/}"
            return 0
        fi
    done
    fail 'No writable Termux temporary directory is available'
}

clear_status() {
    if [ "${CODEX_TERMUX_INSTALL_STATUS_ACTIVE:-0}" -eq 1 ] && [ -t 2 ]; then
        printf '\r\033[2K' >&2
        CODEX_TERMUX_INSTALL_STATUS_ACTIVE=0
    fi
}

say() {
    local message="$*"
    case "$message" in
        *...) ;;
        *) message="$message..." ;;
    esac
    if [ -t 2 ]; then
        printf '\r\033[2K%s' "$message" >&2
        CODEX_TERMUX_INSTALL_STATUS_ACTIVE=1
    else
        printf '%s\n' "$*" >&2
    fi
}

fail() {
    clear_status
    if [ -n "${CODEX_TERMUX_INSTALL_LOG:-}" ] && [ -s "$CODEX_TERMUX_INSTALL_LOG" ]; then
        sed 's/^/  /' "$CODEX_TERMUX_INSTALL_LOG" >&2
    fi
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

need_termux() {
    [ -n "${PREFIX:-}" ] || fail 'PREFIX is not set. Run this inside Termux.'
    [ -x "$PREFIX/bin/pkg" ] || fail 'Termux pkg command not found.'
}

install_dependencies() {
    local missing=() package
    for package in $CODEX_TERMUX_REQUIRED_PACKAGES; do
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

write_source_env_value() {
    local name="$1" value="$2"
    printf 'if [ -z "${%s+x}" ]; then\n' "$name"
    printf '  %s=%q\n' "$name" "$value"
    printf 'fi\n'
    printf 'export %s\n' "$name"
}

configure_wrapper_source() {
    local repo="${1:-}" ref="${2:-main}" token="${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}" config_dir tmp
    if [ -z "$repo" ]; then
        printf 'GitHub repo (OWNER/REPO)> ' >&2
        IFS= read -r repo || fail 'source configuration cancelled'
    fi
    [ -n "$repo" ] || fail 'GitHub repo is required'
    if [ -z "${2+x}" ]; then
        printf 'Git ref [main]> ' >&2
        IFS= read -r ref || fail 'source configuration cancelled'
        ref="${ref:-main}"
    fi
    ref="${ref:-main}"
    if [ -z "$token" ]; then
        printf 'Fine-grained PAT (Contents: read-only)> ' >&2
        if [ -t 0 ]; then
            IFS= read -r -s token || fail 'source configuration cancelled'
            printf '\n' >&2
        else
            IFS= read -r token || fail 'source configuration cancelled'
        fi
    fi
    [ -n "$token" ] || fail 'PAT token is required'
    config_dir="${CODEX_TERMUX_WRAPPER_SOURCE_CONFIG%/*}"
    mkdir -p "$config_dir" || fail "failed to create config directory: $config_dir"
    chmod 700 "$config_dir" || fail "failed to secure config directory: $config_dir"
    tmp="$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG.$$"
    umask 077
    {
        printf '# codex termux wrapper source configuration\n'
        write_source_env_value CODEX_TERMUX_WRAPPER_GIT_REPO "$repo"
        write_source_env_value CODEX_TERMUX_WRAPPER_GIT_REF "$ref"
        write_source_env_value CODEX_TERMUX_WRAPPER_GIT_TOKEN "$token"
    } >"$tmp" || fail 'failed to write wrapper source config'
    chmod 600 "$tmp" || fail 'failed to secure wrapper source config'
    mv "$tmp" "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" || fail 'failed to install wrapper source config'
    printf 'Saved wrapper source config to %s\n' "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" >&2
}

install_managed_runtime() {
    need_termux
    local install_tmp
    install_tmp="$(install_tmp_dir)"
    CODEX_TERMUX_INSTALL_LOG="$(mktemp "$install_tmp/codex-install.XXXXXX")" || fail 'failed to create install log'
    trap 'rm -f "$CODEX_TERMUX_INSTALL_LOG"' EXIT
    say 'checking dependencies'
    install_dependencies
    say 'installing managed runtime'
    CODEX_TERMUX_INSTALL_PRINT_VERSION=0 bash "$ROOT_DIR/bin/install-runtime.sh" install "$@" >"$CODEX_TERMUX_INSTALL_LOG" 2>&1 \
        || fail 'managed runtime install failed'
    say 'verifying public launcher'
    "$PREFIX/bin/codex" version >/dev/null 2>&1 || fail 'public Codex launcher version check failed'
    say 'verifying wrapper diagnostics'
    bash "$ROOT_DIR/bin/install-runtime.sh" doctor --json >>"$CODEX_TERMUX_INSTALL_LOG" 2>&1 \
        || fail 'wrapper doctor verification failed'
    if [ "$CODEX_TERMUX_INSTALL_VERSION_OUTPUT" = "1" ]; then
        clear_status
        "$PREFIX/bin/codex" version
    fi
    clear_status
    [ "$CODEX_TERMUX_INSTALL_OK_OUTPUT" != "1" ] || printf 'ok\n' >&2
}

main() {
    local source_install=1
    if [ "${1:-}" = "source" ]; then
        shift
        case "${1:-}" in
            --save-only|--no-install)
                source_install=0
                shift
                ;;
        esac
        configure_wrapper_source "$@"
        [ "$source_install" = "1" ] || return 0
        set --
    fi
    install_managed_runtime "$@"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    main "$@"
fi
