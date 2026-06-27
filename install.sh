#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CODEX_TERMUX_REQUIRED_PACKAGES="${CODEX_TERMUX_REQUIRED_PACKAGES:-bash curl git nodejs python tar coreutils ca-certificates}"
CODEX_TERMUX_INSTALL_VERSION_OUTPUT="${CODEX_TERMUX_INSTALL_VERSION_OUTPUT:-1}"
CODEX_TERMUX_INSTALL_OK_OUTPUT="${CODEX_TERMUX_INSTALL_OK_OUTPUT:-0}"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="${CODEX_TERMUX_WRAPPER_SOURCE_CONFIG:-$HOME/.config/codex-termux/wrapper-source.env}"
CODEX_TERMUX_BOOTSTRAP_REPO="${CODEX_TERMUX_BOOTSTRAP_REPO:-humtr/codex}"
CODEX_TERMUX_BOOTSTRAP_REF="${CODEX_TERMUX_BOOTSTRAP_REF:-main}"
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

load_wrapper_source_config() {
    [ -r "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" ] || return 0
    # shellcheck disable=SC1090
    . "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG"
}

source_tree_ready() {
    [ -n "$ROOT_DIR" ] &&
        [ -f "$ROOT_DIR/install.sh" ] &&
        [ -f "$ROOT_DIR/bin/install-runtime.sh" ] &&
        [ -f "$ROOT_DIR/lib/codex-termux.sh" ] &&
        [ -f "$ROOT_DIR/tools/build-runtime.py" ] &&
        [ -f "$ROOT_DIR/config/wrapper-version.env" ]
}

find_wrapper_source_tree() {
    local root="$1" candidate
    if (
        ROOT_DIR="$root"
        source_tree_ready
    ); then
        printf '%s\n' "$root"
        return 0
    fi
    while IFS= read -r candidate; do
        candidate="${candidate%/install.sh}"
        if (
            ROOT_DIR="$candidate"
            source_tree_ready
        ); then
            printf '%s\n' "$candidate"
            return 0
        fi
    done <<EOF
$(find "$root" -maxdepth 3 -type f -name install.sh 2>/dev/null)
EOF
    return 1
}

bootstrap_curl_config() {
    local url="$1" out="$2" token="${3:-}" config="$4"
    {
        printf 'url = "%s"\n' "$url"
        printf 'output = "%s"\n' "$out"
        printf 'fail\n'
        printf 'location\n'
        printf 'show-error\n'
        printf 'silent\n'
        [ -z "$token" ] || printf 'header = "Authorization: Bearer %s"\n' "$token"
    } >"$config" || fail 'failed to write bootstrap curl config'
    chmod 600 "$config" || fail 'failed to secure bootstrap curl config'
}

bootstrap_source_tree() {
    local repo ref token tmp archive extract source_dir url curl_config
    source_tree_ready && return 0
    [ "${CODEX_TERMUX_BOOTSTRAPPED:-0}" != "1" ] ||
        fail 'bootstrap did not produce a complete wrapper source tree'
    load_wrapper_source_config
    repo="${CODEX_TERMUX_WRAPPER_GIT_REPO:-$CODEX_TERMUX_BOOTSTRAP_REPO}"
    ref="${CODEX_TERMUX_WRAPPER_GIT_REF:-$CODEX_TERMUX_BOOTSTRAP_REF}"
    token="${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-${CODEX_TERMUX_WRAPPER_RELEASE_TOKEN:-${GITHUB_TOKEN:-}}}"
    [ -n "$repo" ] || fail 'bootstrap repository is not configured'
    [ -n "$ref" ] || ref="main"
    command -v curl >/dev/null 2>&1 || fail 'curl is required for bootstrap install'
    command -v tar >/dev/null 2>&1 || fail 'tar is required for bootstrap install'
    tmp="$(install_tmp_dir)/codex-bootstrap.$$"
    archive="$tmp/source.tar.gz"
    extract="$tmp/extract"
    mkdir -p "$extract" || fail "failed to create bootstrap directory: $tmp"
    url="https://api.github.com/repos/$repo/tarball/$ref"
    curl_config="$tmp/curl.conf"
    bootstrap_curl_config "$url" "$archive" "$token" "$curl_config"
    curl -K "$curl_config" || fail "failed to download wrapper source: $repo@$ref"
    tar -xf "$archive" -C "$extract" || fail 'failed to extract wrapper source archive'
    source_dir="$(find_wrapper_source_tree "$extract")" ||
        fail 'downloaded wrapper source is incomplete'
    CODEX_TERMUX_BOOTSTRAPPED=1 \
    CODEX_TERMUX_BOOTSTRAP_REPO="$repo" \
    CODEX_TERMUX_BOOTSTRAP_REF="$ref" \
    CODEX_TERMUX_WRAPPER_SOURCE_DIR="$source_dir" \
        exec bash "$source_dir/install.sh" "$@"
}

write_source_env_value() {
    local name="$1" value="$2"
    printf 'if [ -z "${%s+x}" ]; then\n' "$name"
    printf '  %s=%q\n' "$name" "$value"
    printf 'fi\n'
    printf 'export %s\n' "$name"
}

write_source_config_file() {
    local repo="$1" ref="$2" token="${3:-}" config_dir tmp
    config_dir="${CODEX_TERMUX_WRAPPER_SOURCE_CONFIG%/*}"
    mkdir -p "$config_dir" || fail "failed to create config directory: $config_dir"
    chmod 700 "$config_dir" || fail "failed to secure config directory: $config_dir"
    tmp="$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG.$$"
    umask 077
    {
        printf '# codex termux wrapper source configuration\n'
        write_source_env_value CODEX_TERMUX_WRAPPER_GIT_REPO "$repo"
        write_source_env_value CODEX_TERMUX_WRAPPER_GIT_REF "$ref"
        [ -z "$token" ] || write_source_env_value CODEX_TERMUX_WRAPPER_GIT_TOKEN "$token"
    } >"$tmp" || fail 'failed to write wrapper source config'
    chmod 600 "$tmp" || fail 'failed to secure wrapper source config'
    mv "$tmp" "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" || fail 'failed to install wrapper source config'
}

configure_wrapper_source() {
    local repo="${1:-}" ref="${2:-main}" token="${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}"
    if [ -z "$token" ]; then
        load_wrapper_source_config
        token="${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}"
    fi
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
    write_source_config_file "$repo" "$ref" "$token"
    printf 'Saved wrapper source config to %s\n' "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" >&2
}

detect_github_repo() {
    local remote
    command -v git >/dev/null 2>&1 || return 1
    git -C "$ROOT_DIR" rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 1
    remote="$(git -C "$ROOT_DIR" config --get remote.origin.url 2>/dev/null || true)"
    case "$remote" in
        https://github.com/*)
            remote="${remote#https://github.com/}"
            ;;
        git@github.com:*)
            remote="${remote#git@github.com:}"
            ;;
        ssh://git@github.com/*)
            remote="${remote#ssh://git@github.com/}"
            ;;
        *)
            return 1
            ;;
    esac
    remote="${remote%.git}"
    case "$remote" in
        */*) printf '%s\n' "$remote" ;;
        *) return 1 ;;
    esac
}

detect_git_ref() {
    local ref
    command -v git >/dev/null 2>&1 || return 1
    ref="$(git -C "$ROOT_DIR" symbolic-ref --quiet --short HEAD 2>/dev/null || true)"
    [ -n "$ref" ] || ref="main"
    printf '%s\n' "$ref"
}

save_current_source_config_if_missing() {
    local repo ref token
    [ "${CODEX_TERMUX_INSTALL_SAVE_SOURCE:-1}" = "1" ] || return 0
    [ ! -r "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" ] || return 0
    repo=""
    ref=""
    if [ "${CODEX_TERMUX_BOOTSTRAPPED:-0}" = "1" ]; then
        repo="${CODEX_TERMUX_BOOTSTRAP_REPO:-}"
        ref="${CODEX_TERMUX_BOOTSTRAP_REF:-}"
    fi
    if [ -z "$repo" ]; then
        repo="$(detect_github_repo || true)"
    fi
    [ -n "$repo" ] || return 0
    if [ -z "$ref" ]; then
        ref="$(detect_git_ref || printf 'main')"
    fi
    token="${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-${GITHUB_TOKEN:-}}"
    write_source_config_file "$repo" "$ref" "$token"
}

install_managed_runtime() {
    need_termux
    local install_tmp
    install_tmp="$(install_tmp_dir)"
    CODEX_TERMUX_INSTALL_LOG="$(mktemp "$install_tmp/codex-install.XXXXXX")" || fail 'failed to create install log'
    trap 'rm -f "$CODEX_TERMUX_INSTALL_LOG"' EXIT
    say 'checking dependencies'
    install_dependencies
    save_current_source_config_if_missing
    say 'installing managed runtime'
    CODEX_TERMUX_WRAPPER_SOURCE_DIR="$ROOT_DIR" \
    CODEX_TERMUX_WRAPPER_GIT_URL= \
    CODEX_TERMUX_WRAPPER_GIT_REPO= \
    CODEX_TERMUX_WRAPPER_RELEASE_URL= \
    CODEX_TERMUX_WRAPPER_RELEASE_REPO= \
    CODEX_TERMUX_WRAPPER_RELEASE_TAG= \
    CODEX_TERMUX_INSTALL_PRINT_VERSION=0 \
        bash "$ROOT_DIR/bin/install-runtime.sh" install "$@" >"$CODEX_TERMUX_INSTALL_LOG" 2>&1 \
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
    bootstrap_source_tree "$@"
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
