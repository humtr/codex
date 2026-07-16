#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${BASH_SOURCE[0]%/*}"
[ "$ROOT_DIR" = "${BASH_SOURCE[0]}" ] && ROOT_DIR="."
ROOT_DIR="$(cd "$ROOT_DIR/.." && pwd)"
# shellcheck disable=SC1091
. "$ROOT_DIR/lib/codex-termux.sh"
CODEX_TERMUX_WRAPPER_SOURCE_TMP=""
CODEX_TERMUX_WRAPPER_GIT_ASKPASS=""
CODEX_TERMUX_WRAPPER_SOURCE_LABEL=""
CODEX_TERMUX_WRAPPER_SOURCE_PLAN_LOADED=0

codex_load_wrapper_source_config

CODEX_TERMUX_WRAPPER_SOURCE_DIR="${CODEX_TERMUX_WRAPPER_SOURCE_DIR:-$ROOT_DIR}"
CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="${CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-$ROOT_DIR/bin/install-runtime.sh}"
CODEX_TERMUX_VERIFIED_MANAGER_LINK="${CODEX_TERMUX_VERIFIED_MANAGER_LINK:-$CODEX_TERMUX_ROOT/verified-manager}"
CODEX_TERMUX_SUPPORT_TRANSACTION_FILE="${CODEX_TERMUX_SUPPORT_TRANSACTION_FILE:-$CODEX_TERMUX_STATE_DIR/support-activation.json}"

usage() {
    # install upstream [VERSION]; install rebuild
    codex_termux_cmd install-usage
}

codex_require_wrapper_source() {
    local source_dir="$1" label="$2" missing
    missing="$(codex_termux_cmd wrapper-source-missing --root "$source_dir")"
    if [ -n "$missing" ]; then
        codex_fail "$label does not contain a valid wrapper source (missing: $(printf '%s' "$missing" | tr '\n' ' '))"
        return 1
    fi
}

codex_load_wrapper_source_plan() {
    local plan_env
    [ "$CODEX_TERMUX_WRAPPER_SOURCE_PLAN_LOADED" = "1" ] && return 0
    codex_normalize_wrapper_source_config
    plan_env="$(codex_termux_cmd wrapper-source-plan-env \
        --repo "${CODEX_TERMUX_WRAPPER_REPO:-}" \
        --ref "${CODEX_TERMUX_WRAPPER_REF:-}" \
        --release-url "${CODEX_TERMUX_WRAPPER_RELEASE_URL:-}" \
        --release-repo "${CODEX_TERMUX_WRAPPER_RELEASE_REPO:-}" \
        --release-tag "${CODEX_TERMUX_WRAPPER_RELEASE_TAG:-}" \
        --local-root "$ROOT_DIR")" || return $?
    eval "$plan_env"
    CODEX_TERMUX_WRAPPER_SOURCE_PLAN_LOADED=1
}

codex_download_wrapper_archive() {
    local source="$1" target="$2"
    local token accept
    local curl_args=(-fsSL)
    if [ -r "$source" ]; then
        cp "$source" "$target"
        return $?
    fi
    token="$(codex_wrapper_auth_token || true)"
    accept="${CODEX_TERMUX_WRAPPER_RELEASE_ACCEPT:-}"
    if [ -z "$accept" ]; then
        case "$source" in
            https://api.github.com/repos/*/releases/assets/*)
                accept="application/octet-stream"
                ;;
        esac
    fi
    [ -z "$token" ] || curl_args+=(-H "Authorization: Bearer $token")
    [ -z "$accept" ] || curl_args+=(-H "Accept: $accept")
    curl "${curl_args[@]}" "$source" -o "$target"
}

codex_fetch_release_wrapper_source() {
    local url tmp archive extract source_dir actual_sha expected_sha
    codex_load_wrapper_source_plan || return $?
    url="${CODEX_WRAPPER_SOURCE_RELEASE_URL:-}"
    [ -n "$url" ] || return 1
    tmp="$(codex_mktemp_dir codex-wrapper-release)" || return 1
    archive="$tmp/wrapper.tar.gz"
    extract="$tmp/extract"
    mkdir -p "$extract"
    codex_download_wrapper_archive "$url" "$archive" || {
        rm -rf "$tmp"
        return 1
    }
    expected_sha="${CODEX_TERMUX_WRAPPER_RELEASE_SHA256:-}"
    if [ -n "$expected_sha" ]; then
        actual_sha="$(codex_sha256 "$archive")"
        [ "$actual_sha" = "$expected_sha" ] || {
            rm -rf "$tmp"
            codex_fail "Wrapper release checksum mismatch"
            return 1
        }
    fi
    codex_termux_cmd validate-tarball --path "$archive" >/dev/null 2>&1 || {
        rm -rf "$tmp"
        codex_fail "Wrapper release archive is unsafe"
        return 1
    }
    tar -xf "$archive" -C "$extract" || {
        rm -rf "$tmp"
        return 1
    }
    source_dir="$(codex_termux_cmd wrapper-source-root --extract-root "$extract")" || {
        rm -rf "$tmp"
        codex_fail "Wrapper release archive does not contain a valid wrapper source"
        return 1
    }
    CODEX_TERMUX_WRAPPER_SOURCE_TMP="$tmp"
    CODEX_TERMUX_WRAPPER_SOURCE_DIR="$source_dir"
    CODEX_TERMUX_WRAPPER_SOURCE_LABEL="${CODEX_WRAPPER_SOURCE_LABEL:-release archive}"
}

codex_prepare_git_askpass() {
    local token="$1" askpass
    [ -n "$token" ] || return 0
    askpass="$2/git-askpass.sh"
    cat >"$askpass" <<'ASKPASS'
#!/bin/sh
case "$1" in
    *Username*) printf '%s\n' "${CODEX_TERMUX_WRAPPER_GIT_USERNAME:-x-access-token}" ;;
    *) printf '%s\n' "$CODEX_TERMUX_WRAPPER_GIT_TOKEN_VALUE" ;;
esac
ASKPASS
    chmod 700 "$askpass"
    CODEX_TERMUX_WRAPPER_GIT_ASKPASS="$askpass"
}

codex_git_clone_wrapper_source() {
    local url ref token tmp checkout
    codex_load_wrapper_source_plan || return $?
    [ "${CODEX_WRAPPER_SOURCE_KIND:-}" = "git" ] || return 1
    url="${CODEX_WRAPPER_SOURCE_GIT_URL:-}"
    [ -n "$url" ] || return 1
    ref="${CODEX_TERMUX_WRAPPER_REF:-}"
    token="$(codex_wrapper_auth_token || true)"
    tmp="$(codex_mktemp_dir codex-wrapper-git)" || return 1
    checkout="$tmp/checkout"
    CODEX_TERMUX_WRAPPER_SOURCE_LABEL="${CODEX_WRAPPER_SOURCE_LABEL:-$url}"
    codex_status "Fetching wrapper source: $CODEX_TERMUX_WRAPPER_SOURCE_LABEL"
    codex_prepare_git_askpass "$token" "$tmp"
    if [ -n "$ref" ]; then
        GIT_TERMINAL_PROMPT=0 \
        GIT_ASKPASS="$CODEX_TERMUX_WRAPPER_GIT_ASKPASS" \
        CODEX_TERMUX_WRAPPER_GIT_TOKEN_VALUE="$token" \
            git clone --quiet --depth 1 --branch "$ref" "$url" "$checkout" || {
            rm -rf "$tmp"
            return 1
        }
    else
        GIT_TERMINAL_PROMPT=0 \
        GIT_ASKPASS="$CODEX_TERMUX_WRAPPER_GIT_ASKPASS" \
        CODEX_TERMUX_WRAPPER_GIT_TOKEN_VALUE="$token" \
            git clone --quiet --depth 1 "$url" "$checkout" || {
            rm -rf "$tmp"
            return 1
        }
    fi
    codex_require_wrapper_source "$checkout" "Wrapper git repository" || {
        rm -rf "$tmp"
        return 1
    }
    CODEX_TERMUX_WRAPPER_SOURCE_TMP="$tmp"
    CODEX_TERMUX_WRAPPER_SOURCE_DIR="$checkout"
}

codex_prepare_fresh_wrapper_source() {
    local kind
    codex_load_wrapper_source_plan || return $?
    kind="${CODEX_WRAPPER_SOURCE_KIND:-}"
    case "$kind" in
        git)
            codex_git_clone_wrapper_source
            ;;
        release)
            codex_fetch_release_wrapper_source
            ;;
        local)
            CODEX_TERMUX_WRAPPER_SOURCE_DIR="${CODEX_WRAPPER_SOURCE_LOCAL_ROOT:-$ROOT_DIR}"
            CODEX_TERMUX_WRAPPER_SOURCE_LABEL="${CODEX_WRAPPER_SOURCE_LABEL:-local $CODEX_TERMUX_WRAPPER_SOURCE_DIR}"
            codex_termux_cmd validate-wrapper-source --root "$CODEX_TERMUX_WRAPPER_SOURCE_DIR" >/dev/null
            ;;
        *)
            codex_fail "Unknown wrapper source kind: $kind"
            return 1
            ;;
    esac
}

codex_cleanup_fresh_wrapper_source() {
    [ -z "$CODEX_TERMUX_WRAPPER_SOURCE_TMP" ] || rm -rf "$CODEX_TERMUX_WRAPPER_SOURCE_TMP"
    CODEX_TERMUX_WRAPPER_SOURCE_TMP=""
    CODEX_TERMUX_WRAPPER_SOURCE_LABEL=""
    CODEX_TERMUX_WRAPPER_SOURCE_PLAN_LOADED=0
}

codex_copy_wrapper_source_snapshot() {
    local source_dir="$1" target_dir="$2"
    if [ "$(cd "$source_dir" && pwd)" = "$(mkdir -p "$target_dir" && cd "$target_dir" && pwd)" ]; then
        return 0
    fi
    codex_rm_rf_managed "$target_dir" || return $?
    mkdir -p "$target_dir" || return $?
    cp "$source_dir/install.sh" "$target_dir/install.sh"
    cp -R "$source_dir/bin" "$target_dir/bin"
    cp -R "$source_dir/lib" "$target_dir/lib"
    cp -R "$source_dir/tools" "$target_dir/tools"
    cp -R "$source_dir/config" "$target_dir/config"
    [ ! -f "$source_dir/README.md" ] || cp "$source_dir/README.md" "$target_dir/README.md"
}

codex_write_managed_shell() {
    mkdir -p "$CODEX_TERMUX_MANAGER_DIR"
    cat >"$CODEX_TERMUX_MANAGED_SHELL.$$" <<MANAGED
#!$CODEX_TERMUX_PREFIX/bin/bash
# codex termux managed shell
set -euo pipefail
export CODEX_TERMUX_INSTALL_RUNTIME_SOURCE="\${CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-$CODEX_TERMUX_SOURCE_DIR/bin/install-runtime.sh}"
# shellcheck disable=SC1091
. "$CODEX_TERMUX_MANAGER_DIR/lib.sh"
codex_main "\$@"
MANAGED
    chmod 755 "$CODEX_TERMUX_MANAGED_SHELL.$$"
    mv "$CODEX_TERMUX_MANAGED_SHELL.$$" "$CODEX_TERMUX_MANAGED_SHELL"
}

codex_remove_python_bytecode() {
    local python_root="$1"
    [ -d "$python_root" ] || return 0
    find "$python_root" \( -type d -name __pycache__ -o -type f -name '*.pyc' \) -exec rm -rf {} +
}

codex_check_manager_python() {
    local manager_package="$CODEX_TERMUX_MANAGER_DIR/codex_termux"
    codex_remove_python_bytecode "$manager_package"
    python3 - "$manager_package" <<'PYTHON'
import pathlib
import sys

package = pathlib.Path(sys.argv[1])
for path in sorted(package.glob("*.py")):
    source = path.read_text(encoding="utf-8")
    compile(source, str(path), "exec")
PYTHON
    codex_remove_python_bytecode "$manager_package"
}

codex_install_support_files() {
    local wrapper_commit source_dir="$CODEX_TERMUX_WRAPPER_SOURCE_DIR" package_root installed_at status=0
    codex_require_wrapper_source "$source_dir" "Invalid wrapper source: $source_dir" || return $?
    mkdir -p "$CODEX_TERMUX_ROOT" "$CODEX_TERMUX_STATE_DIR"
    wrapper_commit="$(codex_termux_cmd wrapper-source-commit --root "$source_dir")"
    installed_at="$(date -Is)"
    package_root="$(codex_termux_package_root)" || return 1

    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" \
        python3 -B - \
            "$source_dir" \
            "$CODEX_TERMUX_ROOT" \
            "$CODEX_TERMUX_MANAGER_DIR" \
            "$CODEX_TERMUX_VERIFIED_MANAGER_LINK" \
            "$CODEX_TERMUX_STATE_DIR" \
            "$CODEX_TERMUX_PREFIX" \
            "$installed_at" \
            "$wrapper_commit" <<'PYTHON' || return $?
from pathlib import Path
import json
import sys

from codex_termux import source

(
    source_root,
    wrapper_root,
    manager_link,
    verified_manager_link,
    state_dir,
    prefix,
    installed_at,
    wrapper_commit,
) = sys.argv[1:]
result = source.prepare_support_install(
    source_root=Path(source_root),
    wrapper_root=Path(wrapper_root),
    manager_link=Path(manager_link),
    verified_manager_link=Path(verified_manager_link),
    state_dir=Path(state_dir),
    prefix=Path(prefix),
    installed_at=installed_at,
    wrapper_commit=wrapper_commit,
)
print(json.dumps(result.to_dict(), ensure_ascii=True, sort_keys=True))
PYTHON

    if [ "${CODEX_TERMUX_INSTALL_FAIL_AFTER_MANAGER_SWITCH:-0}" = "1" ]; then
        status=97
    elif ! codex_prepare_system_config; then
        status=$?
        [ "$status" -ne 0 ] || status=1
    fi

    if [ "$status" -ne 0 ]; then
        PYTHONDONTWRITEBYTECODE=1 \
        PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" \
            python3 -B - "$CODEX_TERMUX_SUPPORT_TRANSACTION_FILE" <<'PYTHON' || true
from pathlib import Path
import sys
from codex_termux import source
source.rollback_support_install(Path(sys.argv[1]))
PYTHON
        codex_prepare_system_config >/dev/null 2>&1 || true
        codex_fail "Support manager activation failed; restored the previous manager"
        return "$status"
    fi

    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" \
        python3 -B - "$CODEX_TERMUX_SUPPORT_TRANSACTION_FILE" <<'PYTHON' || {
from pathlib import Path
import sys
from codex_termux import source
source.commit_support_install(Path(sys.argv[1]))
PYTHON
        PYTHONDONTWRITEBYTECODE=1 \
        PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" \
            python3 -B - "$CODEX_TERMUX_SUPPORT_TRANSACTION_FILE" <<'PYTHON' || true
from pathlib import Path
import sys
from codex_termux import source
source.rollback_support_install(Path(sys.argv[1]))
PYTHON
        codex_fail "Support manager commit failed; restored the previous manager"
        return 1
    }
}

codex_build_launcher() {
    clang -O2 -Wall -Wextra -o "$1" "$CODEX_TERMUX_WRAPPER_SOURCE_DIR/tools/codex-launcher.c"
}

codex_prepare_launcher_slot() {
    local public="$1" backup base
    mkdir -p "${public%/*}" "$CODEX_TERMUX_BACKUP_DIR"
    if [ -d "$public" ] && [ ! -L "$public" ]; then
        codex_fail "refusing to replace launcher directory $public"
        return 1
    fi
    if [ -e "$public" ] || [ -L "$public" ]; then
        if codex_termux_cmd file-has-marker \
            --path "$public" \
            --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER"; then
            return 0
        fi
        base="$(basename "$public")"
        backup="$CODEX_TERMUX_BACKUP_DIR/$base.$(date +%Y%m%d-%H%M%S).bak"
        cp -Pp "$public" "$backup"
    fi
}

codex_write_shell_launcher() {
    local public="$1" tmp
    tmp="${public}.new.$$"
    mkdir -p "${public%/*}"
    cat >"$tmp" <<LAUNCHER
#!/bin/sh
# $CODEX_TERMUX_MANAGED_LAUNCHER_MARKER
exec "$CODEX_TERMUX_MANAGED_SHELL" "\$@"
LAUNCHER
    chmod 755 "$tmp"
    if ! codex_prepare_launcher_slot "$public" || ! mv -f "$tmp" "$public"; then
        rm -f "$tmp"
        return 1
    fi
}

codex_write_compiled_launcher() {
    local public="$1" tmp
    tmp="${public}.new.$$"
    mkdir -p "${public%/*}"
    codex_build_launcher "$tmp"
    if ! codex_termux_cmd file-has-marker \
        --path "$tmp" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER"; then
        rm -f "$tmp"
        codex_fail "compiled launcher missing managed marker"
        return 1
    fi
    chmod 755 "$tmp"
    if ! codex_prepare_launcher_slot "$public" || ! mv -f "$tmp" "$public"; then
        rm -f "$tmp"
        return 1
    fi
}

codex_install_launchers() {
    if command -v clang >/dev/null 2>&1; then
        codex_write_compiled_launcher "$CODEX_TERMUX_PUBLIC_CODEX"
    else
        codex_write_shell_launcher "$CODEX_TERMUX_PUBLIC_CODEX"
    fi
}

codex_install_surface_run() {
    local message="$1" success_message="${2:-}" command="$3" status=0
    shift 3
    if [ "${CODEX_TERMUX_INSTALL_SURFACE:-1}" = "0" ]; then
        "$command" "$@"
        return $?
    fi
    codex_status "$message"
    "$command" "$@" || status=$?
    if [ "$status" -eq 0 ]; then
        if [ -n "$success_message" ]; then
            codex_say "$success_message"
        elif [ "${CODEX_TERMUX_INSTALL_PRINT_VERSION:-1}" = "0" ]; then
            codex_status_clear
        else
            codex_version || status=$?
        fi
    else
        codex_status_clear
    fi
    return "$status"
}

codex_install_support_and_launchers() {
    codex_install_support_files &&
    codex_install_launchers
}

codex_with_fresh_wrapper_source() {
    local command="$1" status=0
    shift
    codex_prepare_fresh_wrapper_source || return $?
    [ -z "$CODEX_TERMUX_WRAPPER_SOURCE_LABEL" ] || codex_status "Using wrapper source: $CODEX_TERMUX_WRAPPER_SOURCE_LABEL"
    "$command" "$@" || status=$?
    codex_cleanup_fresh_wrapper_source
    return "$status"
}

codex_install_full_core() {
    local version="${1:-}"
    codex_validate_runtime_retention &&
    codex_install_support_and_launchers &&
    codex_runtime_install_upstream "$version" &&
    codex_refresh_runtime_metadata
}

codex_install_support_core() {
    codex_install_support_and_launchers
}

codex_install_rebuild_core() {
    codex_validate_runtime_retention &&
    codex_install_support_and_launchers &&
    codex_runtime_install_cached &&
    codex_refresh_runtime_metadata
}

codex_install_full_unlocked() {
    codex_with_fresh_wrapper_source codex_install_full_core "${1:-}"
}

codex_install_support_unlocked() {
    codex_with_fresh_wrapper_source codex_install_support_core
}

codex_install_rebuild_unlocked() {
    codex_with_fresh_wrapper_source codex_install_rebuild_core
}

codex_install_run_plan() {
    local command="$1" plan_env action version message success_message exit_code error
    shift
    plan_env="$(codex_termux_cmd install-plan-env --command "$command" -- "$@")" || return $?
    eval "$plan_env"
    action="${CODEX_INSTALL_PLAN_ACTION:-}"
    version="${CODEX_INSTALL_PLAN_VERSION:-}"
    message="${CODEX_INSTALL_PLAN_SURFACE_MESSAGE:-}"
    success_message="${CODEX_INSTALL_PLAN_SUCCESS_MESSAGE:-}"
    case "$action" in
        usage)
            usage
            ;;
        error)
            exit_code="${CODEX_INSTALL_PLAN_EXIT_CODE:-1}"
            error="${CODEX_INSTALL_PLAN_ERROR:-Unknown install plan error}"
            codex_fail "$error"
            return "$exit_code"
            ;;
        install_full)
            codex_install_surface_run "$message" "$success_message" codex_with_lock codex_install_full_unlocked "$version"
            ;;
        support)
            codex_install_surface_run "$message" "$success_message" codex_with_lock codex_install_support_unlocked
            ;;
        upstream)
            codex_install_surface_run "$message" "$success_message" codex_runtime_install_upstream "$version"
            ;;
        rebuild)
            codex_install_surface_run "$message" "$success_message" codex_with_lock codex_install_rebuild_unlocked
            ;;
        repair)
            codex_install_surface_run "$message" "$success_message" codex_with_lock codex_repair_core_unlocked
            ;;
        *)
            codex_fail "Unknown install plan action: $action"
            return 1
            ;;
    esac
}

main() {
    case "${1:-install}" in
        install)
            shift || true
            codex_install_run_plan install "$@"
            ;;
        repair)
            shift || true
            codex_install_run_plan repair "$@"
            ;;
        update)
            shift || true
            codex_install_run_plan update "$@"
            ;;
        setup)
            printf 'The setup command is reserved for configuration. Use install, update, or repair.\n' >&2
            exit 2
            ;;
        remove)
            codex_remove
            ;;
        doctor)
            shift || true
            codex_termux_doctor "$@"
            ;;
        -h|--help|help)
            usage
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    main "$@"
fi
