#!/usr/bin/env bash
set -u

CODEX_TERMUX_SHELL_DIR="${BASH_SOURCE[0]%/*}"
[ "$CODEX_TERMUX_SHELL_DIR" = "${BASH_SOURCE[0]}" ] && CODEX_TERMUX_SHELL_DIR="."
CODEX_TERMUX_SHELL_DIR="$(cd "$CODEX_TERMUX_SHELL_DIR" && pwd)"

CODEX_TERMUX_HOME="${CODEX_TERMUX_HOME:-$HOME}"
CODEX_TERMUX_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_TERMUX_ROOT="${CODEX_TERMUX_ROOT:-$CODEX_TERMUX_HOME/.local/lib/codex/termux}"
CODEX_TERMUX_MANAGER_DIR="${CODEX_TERMUX_MANAGER_DIR:-$CODEX_TERMUX_ROOT/manager}"
CODEX_TERMUX_SOURCE_DIR="${CODEX_TERMUX_SOURCE_DIR:-$CODEX_TERMUX_MANAGER_DIR/source}"
CODEX_TERMUX_RAW_DIR="${CODEX_TERMUX_RAW_DIR:-$CODEX_TERMUX_ROOT/raw}"
CODEX_TERMUX_RAW_VENDOR="${CODEX_TERMUX_RAW_VENDOR:-$CODEX_TERMUX_RAW_DIR/vendor/aarch64-unknown-linux-musl}"
CODEX_TERMUX_RUNTIME_DIR="${CODEX_TERMUX_RUNTIME_DIR:-$CODEX_TERMUX_ROOT/current}"
CODEX_TERMUX_CURRENT_LINK="${CODEX_TERMUX_CURRENT_LINK:-$CODEX_TERMUX_RUNTIME_DIR}"
CODEX_TERMUX_VERIFIED_LINK="${CODEX_TERMUX_VERIFIED_LINK:-$CODEX_TERMUX_ROOT/verified}"
CODEX_TERMUX_RUNTIME="${CODEX_TERMUX_RUNTIME:-$CODEX_TERMUX_RUNTIME_DIR/codex}"
CODEX_TERMUX_MANAGED_SHELL="${CODEX_TERMUX_MANAGED_SHELL:-$CODEX_TERMUX_MANAGER_DIR/managed.sh}"
CODEX_TERMUX_STATE_DIR="${CODEX_TERMUX_STATE_DIR:-$CODEX_TERMUX_HOME/.local/share/codex/termux}"
CODEX_TERMUX_PROFILE_ROOT="${CODEX_TERMUX_PROFILE_ROOT:-$CODEX_TERMUX_HOME/.codex-profiles}"
CODEX_TERMUX_STATE_FILE="${CODEX_TERMUX_STATE_FILE:-$CODEX_TERMUX_STATE_DIR/state.json}"
CODEX_TERMUX_REGISTRY_FILE="${CODEX_TERMUX_REGISTRY_FILE:-$CODEX_TERMUX_STATE_DIR/registry.json}"
CODEX_TERMUX_STORE_DIR="${CODEX_TERMUX_STORE_DIR:-$CODEX_TERMUX_ROOT/store}"
CODEX_TERMUX_RUNTIME_STORE_DIR="${CODEX_TERMUX_RUNTIME_STORE_DIR:-$CODEX_TERMUX_STORE_DIR/runtime}"
CODEX_TERMUX_RAW_STORE_DIR="${CODEX_TERMUX_RAW_STORE_DIR:-$CODEX_TERMUX_STORE_DIR/raw}"
CODEX_TERMUX_BACKUP_DIR="${CODEX_TERMUX_BACKUP_DIR:-$CODEX_TERMUX_STATE_DIR/backups}"
CODEX_TERMUX_DOCTOR_DIR="${CODEX_TERMUX_DOCTOR_DIR:-$CODEX_TERMUX_STATE_DIR/doctor}"
CODEX_TERMUX_LOCK_FILE="${CODEX_TERMUX_LOCK_FILE:-$CODEX_TERMUX_STATE_DIR/termux.lock}"
CODEX_TERMUX_LOCK_WAIT_SECONDS="${CODEX_TERMUX_LOCK_WAIT_SECONDS:-30}"
CODEX_TERMUX_RESOLV_CONF="${CODEX_TERMUX_RESOLV_CONF:-$CODEX_TERMUX_PREFIX/etc/resolv.conf}"
CODEX_TERMUX_TMPDIR="${CODEX_TERMUX_TMPDIR:-$CODEX_TERMUX_PREFIX/tmp}"
CODEX_TERMUX_SYSTEM_CONFIG_DIR="${CODEX_TERMUX_SYSTEM_CONFIG_DIR:-$CODEX_TERMUX_STATE_DIR/system-config}"
CODEX_TERMUX_TURN_NOTIFY="${CODEX_TERMUX_TURN_NOTIFY:-$CODEX_TERMUX_MANAGER_DIR/codex-turn-notify.sh}"
CODEX_TERMUX_NOTIFY_DIR="${CODEX_TERMUX_NOTIFY_DIR:-$CODEX_TERMUX_STATE_DIR/notify}"
CODEX_TERMUX_NOTIFY_CONFIG="${CODEX_TERMUX_NOTIFY_CONFIG:-$CODEX_TERMUX_NOTIFY_DIR/config.env}"
CODEX_TERMUX_NOTIFY_GROUP="${CODEX_TERMUX_NOTIFY_GROUP:-codex-turns}"
CODEX_TERMUX_NOTIFY_CONTENT_CHARS="${CODEX_TERMUX_NOTIFY_CONTENT_CHARS:-140}"
CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES="${CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES:-0}"
CODEX_TERMUX_NOTIFY_TOAST_SHORT="${CODEX_TERMUX_NOTIFY_TOAST_SHORT:-0}"
CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND="${CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND:-}"
CODEX_TERMUX_NOTIFY_TOAST_COLOR="${CODEX_TERMUX_NOTIFY_TOAST_COLOR:-}"
CODEX_TERMUX_NOTIFY_PRETOOLUSE="${CODEX_TERMUX_NOTIFY_PRETOOLUSE:-0}"
CODEX_TERMUX_NOTIFY_HOOKS="${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}"
CODEX_TERMUX_NOTIFY_TOAST_GRAVITY="${CODEX_TERMUX_NOTIFY_TOAST_GRAVITY:-top}"
CODEX_TERMUX_CERT_FILE="${CODEX_TERMUX_CERT_FILE:-$CODEX_TERMUX_PREFIX/etc/tls/cert.pem}"
CODEX_TERMUX_CERT_DIR="${CODEX_TERMUX_CERT_DIR:-$CODEX_TERMUX_PREFIX/etc/tls/certs}"
CODEX_TERMUX_PACKAGE_SPEC_DEFAULT="${CODEX_TERMUX_PACKAGE_SPEC_DEFAULT:-@openai/codex@linux-arm64}"
CODEX_TERMUX_MANAGED_LAUNCHER_MARKER="${CODEX_TERMUX_MANAGED_LAUNCHER_MARKER:-codex termux managed launcher}"
CODEX_TERMUX_PUBLIC_CODEX="${CODEX_TERMUX_PUBLIC_CODEX:-$CODEX_TERMUX_PREFIX/bin/codex}"
CODEX_TERMUX_RUNTIME_BUILDER="${CODEX_TERMUX_RUNTIME_BUILDER:-$CODEX_TERMUX_MANAGER_DIR/build-runtime.py}"
CODEX_TERMUX_AUTO_UPDATE="${CODEX_TERMUX_AUTO_UPDATE:-1}"
CODEX_TERMUX_AUTO_UPDATE_MODE="${CODEX_TERMUX_AUTO_UPDATE_MODE:-prompt}"
CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS="${CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS:-21600}"
CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS="${CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS:-4}"
CODEX_TERMUX_AUTO_UPDATE_STAMP="${CODEX_TERMUX_AUTO_UPDATE_STAMP:-$CODEX_TERMUX_STATE_DIR/last-auto-update-check}"
CODEX_TERMUX_AUTO_UPDATE_PENDING="${CODEX_TERMUX_AUTO_UPDATE_PENDING:-$CODEX_TERMUX_STATE_DIR/pending-auto-update-version}"
CODEX_TERMUX_AUTO_UPDATE_FAILED="${CODEX_TERMUX_AUTO_UPDATE_FAILED:-$CODEX_TERMUX_STATE_DIR/failed-auto-update}"
CODEX_TERMUX_UPSTREAM_TIME_CACHE="${CODEX_TERMUX_UPSTREAM_TIME_CACHE:-$CODEX_TERMUX_STATE_DIR/upstream-time-cache.tsv}"
CODEX_TERMUX_LAST_PROFILE_FILE="${CODEX_TERMUX_LAST_PROFILE_FILE:-$CODEX_TERMUX_STATE_DIR/last-profile}"
CODEX_TERMUX_RUNTIME_RETENTION="${CODEX_TERMUX_RUNTIME_RETENTION:-3}"
CODEX_TERMUX_PATCH_POLICY="${CODEX_TERMUX_PATCH_POLICY:-termux-fd-remap-v1}"

CODEX_STATUS_ACTIVE=0

codex_status_clear() {
    if [ "${CODEX_STATUS_ACTIVE:-0}" -eq 1 ] && [ -t 2 ]; then
        printf '\r\033[2K' >&2
        CODEX_STATUS_ACTIVE=0
    fi
}

codex_status() {
    local message="$*"
    case "$message" in
        *...) ;;
        *) message="$message..." ;;
    esac
    if [ -t 2 ]; then
        printf '\r\033[2K%s' "$message" >&2
        CODEX_STATUS_ACTIVE=1
    else
        printf '%s\n' "$message" >&2
    fi
}

codex_say() {
    codex_status_clear
    printf '%s\n' "$*" >&2
}

codex_selection_cancelled() {
    codex_say "$(codex_ui_text_get selection_cancelled)"
}

codex_fail() {
    codex_status_clear
    printf 'Error: %s\n' "$*" >&2
    return 1
}

codex_ui_enabled() {
    [ -t 2 ] && [ -z "${NO_COLOR:-}" ]
}

codex_ui_color() {
    local code="$1" text="$2"
    if codex_ui_enabled; then
        printf '\033[%sm%s\033[0m' "$code" "$text"
    else
        printf '%s' "$text"
    fi
}

codex_ui_dim() {
    codex_ui_color "2" "$1"
}

codex_ui_number() {
    codex_ui_color "36" "$(printf '%2s.' "$1")"
}

codex_ui_badge() {
    local kind="$1" text code
    case "$kind" in
        active)
            text=" 🟢 active "
            code="42;30"
            ;;
        current)
            text=" 🟢 current "
            code="42;30"
            ;;
        cached)
            text=" 📦 cached "
            code="44;97"
            ;;
        run)
            text=" ▶ run "
            code="46;30"
            ;;
        install)
            text=" ⬇ install "
            code="43;30"
            ;;
        update)
            text=" ⬇ update "
            code="43;30"
            ;;
        latest)
            text=" ⬆ latest "
            code="45;97"
            ;;
        recent)
            text=" 🕘 recent "
            code="46;30"
            ;;
        keep)
            text=" ↵ keep "
            code="100;97"
            ;;
        *)
            text=" $kind "
            code="2"
            ;;
    esac
    codex_ui_color "$code" "$text"
}

codex_ui_menu_header() {
    local title="$1" subtitle="${2:-}"
    codex_status_clear
    printf '%s\n' "$title" >&2
    if [ -n "$subtitle" ]; then
        printf '%s\n' "$(codex_ui_dim "$subtitle")" >&2
    fi
}

codex_ui_menu_note() {
    [ -n "${1:-}" ] || return 0
    printf '%s\n' "$(codex_ui_dim "$1")" >&2
}

codex_ui_menu_row() {
    local key="$1" label="$2"
    shift 2 || true
    printf '  %s %s' "$(codex_ui_number "$key")" "$label" >&2
    while [ "$#" -gt 0 ]; do
        [ -n "$1" ] && printf ' %s' "$1" >&2
        shift
    done
    printf '\n' >&2
}

codex_ui_prompt() {
    codex_ui_dim "$1"
}

codex_ui_version_row() {
    local label="$1" value="$2"
    printf '%-16s %s\n' "$label" "$value" >&2
}

codex_ui_separator() {
    local width="${1:-61}" line
    line="$(printf '%*s' "$width" '')"
    line="${line// /─}"
    printf '%s\n' "$(codex_ui_dim "$line")" >&2
}

codex_ui_text() {
    local key="$1"
    shift || true
    case "$key" in
        selection_cancelled) printf 'Selection cancelled.\n' ;;
        profile_create_cancelled) printf 'Profile creation cancelled.\n' ;;
        choose_profile_title) printf 'Choose profile\n' ;;
        choose_profile_subtitle) printf 'Select CODEX_HOME target\n' ;;
        choose_profile_prompt) printf 'Choose profile > \n' ;;
        choose_profile_more) printf '  (More options: codex profile NAME)\n' ;;
        choose_runtime_prompt) printf 'Choose runtime > \n' ;;
        update_complete_title) printf 'Update complete\n' ;;
        update_ready_subtitle) printf 'Codex %s is ready\n' "$1" ;;
        update_available_title) printf 'Update available\n' ;;
        launch_now_prompt) printf 'Launch now [y/N]> \n' ;;
        apply_update_prompt) printf 'Apply update [y/N]> \n' ;;
        launch_label) printf 'launch Codex\n' ;;
        done_label) printf 'done\n' ;;
        current_kept) printf 'Kept current runtime (%s).\n' "$1" ;;
        create_profile_prompt) printf "Create profile '%s' [y/N]> \n" "$1" ;;
        created_profile) printf 'Created profile %s.\n' "$1" ;;
        installed_codex) printf 'Installed Codex %s\n' "$1" ;;
        rebuilt_cached_runtime) printf 'Rebuilt runtime from cached raw package (%s)\n' "$1" ;;
        update_failed_continue) printf 'Update failed. Continuing with %s.\n' "$1" ;;
        restored_verified) printf 'Restored the active runtime from the verified copy.\n' ;;
        restored_backup) printf 'Restored %s from %s.\n' "$1" "$2" ;;
        removed_runtime) printf 'Removed the managed runtime. State remains at %s.\n' "$1" ;;
        invalid_profile) printf 'Invalid profile name: %s\n' "$1" ;;
        missing_profile) printf 'Profile does not exist: %s\n' "$1" ;;
        profile_arg_error) printf 'Profile %s does not take arguments\n' "$1" ;;
        setup_reserved) printf 'codex setup is reserved for configuration. Use codex install, update, repair, or notify.\n' ;;
        doctor_wrapper_title) printf 'Wrapper doctor\n' ;;
        session_stub) printf 'codex session is reserved for the upcoming cross-profile session picker.\n' ;;
        *)
            return 1
            ;;
    esac
}

codex_ui_step_text() {
    local key="$1"
    shift || true
    case "$key" in
        fetch_package) printf 'Fetching %s\n' "$1" ;;
        validate_archive) printf 'Validating package archive\n' ;;
        unpack_archive) printf 'Unpacking package archive\n' ;;
        stage_raw) printf 'Staging raw package\n' ;;
        build_runtime) printf 'Building patched runtime\n' ;;
        assemble_runtime) printf 'Assembling runtime bundle\n' ;;
        smoke_test_runtime) printf 'Smoke-testing runtime\n' ;;
        activate_runtime) printf 'Activating runtime\n' ;;
        update_runtime) printf 'Updating Codex %s -> %s\n' "$1" "$2" ;;
        switch_runtime) printf 'Switching to Codex %s\n' "$1" ;;
        launch_codex) printf 'Launching Codex %s\n' "$1" ;;
        install_runtime) printf 'Installing wrapper support and fresh upstream runtime\n' ;;
        rebuild_runtime) printf 'Rebuilding wrapper support with cached raw runtime\n' ;;
        repair_runtime) printf 'Repairing runtime from the cached raw package\n' ;;
        repair_support) printf 'Repairing wrapper support and launcher\n' ;;
        repair_metadata) printf 'Repairing runtime metadata\n' ;;
        rebuild_cached_runtime) printf 'Rebuilding runtime from the cached raw package\n' ;;
        open_profile) printf 'Opening profile %s\n' "$1" ;;
        *)
            return 1
            ;;
    esac
}

codex_ui_text_get() {
    codex_ui_text "$@" | tr -d '\n'
}

codex_ui_step() {
    local message
    message="$(codex_ui_step_text "$@")" || return 1
    codex_status "${message%$'\n'}"
}

codex_display_version() {
    local version="${1:-unknown}"
    case "$version" in
        *-linux-arm64)
            printf '%s\n' "${version%-linux-arm64}"
            ;;
        *)
            printf '%s\n' "$version"
            ;;
    esac
}

codex_parent_dir() {
    local path="$1"
    path="${path%/*}"
    [ -n "$path" ] || path="."
    printf '%s\n' "$path"
}

codex_strip_trailing_slashes() {
    local path="$1"
    while [ "${#path}" -gt 1 ] && [ "${path%/}" != "$path" ]; do
        path="${path%/}"
    done
    printf '%s\n' "$path"
}

codex_tmp_dir() {
    local candidate
    for candidate in "${CODEX_TERMUX_TMPDIR:-}" "${TMPDIR:-}" "$CODEX_TERMUX_PREFIX/tmp"; do
        [ -n "$candidate" ] || continue
        case "$candidate" in
            /tmp) continue ;;
            /*) ;;
            *) continue ;;
        esac
        if mkdir -p "$candidate" 2>/dev/null && [ -d "$candidate" ] && [ -w "$candidate" ]; then
            printf '%s\n' "$(codex_strip_trailing_slashes "$candidate")"
            return 0
        fi
    done
    codex_fail "No writable Termux temporary directory is available"
    return 1
}

codex_mktemp_dir() {
    local prefix="${1:-codex-tmp}" tmpdir
    tmpdir="$(codex_tmp_dir)" || return $?
    mktemp -d "$tmpdir/$prefix.XXXXXX"
}

codex_mktemp_file() {
    local prefix="${1:-codex-tmp}" tmpdir
    tmpdir="$(codex_tmp_dir)" || return $?
    mktemp "$tmpdir/$prefix.XXXXXX"
}

codex_path_is_within() {
    local path root
    path="$(codex_strip_trailing_slashes "$1")"
    root="$(codex_strip_trailing_slashes "$2")"
    [ -n "$root" ] && [ "$root" != "/" ] || return 1
    [ "$path" = "$root" ] && return 0
    case "$path" in
        "$root"/*) return 0 ;;
        *) return 1 ;;
    esac
}

codex_assert_safe_path() {
    local path label home prefix tmpdir
    path="$(codex_strip_trailing_slashes "$1")"
    label="${2:-path}"
    [ -n "$path" ] || {
        codex_fail "$label must not be empty"
        return 64
    }
    case "$path" in
        /*) ;;
        *)
            codex_fail "$label must be absolute: $path"
            return 64
            ;;
    esac
    home="$(codex_strip_trailing_slashes "${CODEX_TERMUX_HOME:-${HOME:-}}")"
    prefix="$(codex_strip_trailing_slashes "$CODEX_TERMUX_PREFIX")"
    tmpdir="$(codex_strip_trailing_slashes "$(codex_tmp_dir 2>/dev/null || printf '%s\n' "$CODEX_TERMUX_PREFIX/tmp")")"
    case "$path" in
        /|"$home"|"$prefix"|"$tmpdir"|/tmp)
            codex_fail "$label points to an unsafe path: $path"
            return 64
            ;;
    esac
}

codex_assert_managed_tree_target() {
    local path label root state
    path="$(codex_strip_trailing_slashes "$1")"
    label="${2:-managed tree target}"
    root="$(codex_strip_trailing_slashes "$CODEX_TERMUX_ROOT")"
    state="$(codex_strip_trailing_slashes "$CODEX_TERMUX_STATE_DIR")"
    codex_assert_safe_path "$path" "$label" || return $?
    codex_assert_safe_path "$root" CODEX_TERMUX_ROOT || return $?
    codex_assert_safe_path "$state" CODEX_TERMUX_STATE_DIR || return $?
    if codex_path_is_within "$path" "$root" || codex_path_is_within "$path" "$state"; then
        return 0
    fi
    codex_fail "$label is outside managed wrapper paths: $path"
    return 64
}

codex_rm_rf_managed() {
    local path
    for path in "$@"; do
        codex_assert_managed_tree_target "$path" "destructive target" || return $?
    done
    rm -rf "$@"
}

codex_sha256() {
    codex_termux_cmd hash-file --path "$1"
}

codex_now() {
    date -Is
}

codex_termux_package_root() {
    local source_dir source_root package_root=""
    source_dir="${BASH_SOURCE[0]%/*}"
    [ "$source_dir" = "${BASH_SOURCE[0]}" ] && source_dir="."
    source_dir="$(cd "$source_dir" && pwd)"
    source_root="$(cd "$source_dir/.." && pwd)"
    if [ -d "$source_root/tools/codex_termux" ]; then
        package_root="$source_root/tools"
    elif [ -n "${ROOT_DIR:-}" ] && [ -d "$ROOT_DIR/tools/codex_termux" ]; then
        package_root="$ROOT_DIR/tools"
    elif [ -d "$CODEX_TERMUX_MANAGER_DIR/codex_termux" ]; then
        package_root="$CODEX_TERMUX_MANAGER_DIR"
    else
        codex_fail "Internal helper package is unavailable"
        return 1
    fi
    printf '%s\n' "$package_root"
}

codex_termux_cmd() {
    local package_root
    package_root="$(codex_termux_package_root)" || return 1
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" python3 -B -m codex_termux.cli "$@"
}

codex_termux_activation_cmd() {
    local action="$1" shell_lib="${BASH_SOURCE[0]}" wrapper_version wrapper_commit
    shift
    wrapper_version="$(codex_current_wrapper_version)" || return 1
    wrapper_commit="$(codex_current_wrapper_commit)" || return 1
    codex_termux_cmd "$action" \
        --current-link "$CODEX_TERMUX_CURRENT_LINK" \
        --verified-link "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw-link "$CODEX_TERMUX_RAW_DIR" \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_TERMUX_RAW_STORE_DIR" \
        --wrapper-version "$wrapper_version" \
        --wrapper-commit "$wrapper_commit" \
        --updated-at "$(codex_now)" \
        --shell-bin "${BASH:-bash}" \
        --shell-lib "$shell_lib" \
        --home "$CODEX_TERMUX_HOME" \
        --prefix "$CODEX_TERMUX_PREFIX" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --resolv-conf "$CODEX_TERMUX_RESOLV_CONF" \
        --cert-file "$CODEX_TERMUX_CERT_FILE" \
        --cert-dir "$CODEX_TERMUX_CERT_DIR" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        "$@"
}

codex_file_has_marker() {
    local path="$1"
    [ -e "$path" ] || return 1
    grep -a -q "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER" "$path" 2>/dev/null
}

codex_lock_is_held() {
    local pid="${BASHPID:-$$}"
    [ "${CODEX_TERMUX_LOCK_HELD:-0}" = "1" ] || return 1
    [ "$(readlink "/proc/$pid/fd/9" 2>/dev/null || true)" = "$CODEX_TERMUX_LOCK_FILE" ]
}

codex_with_lock() {
    local cmd="$1"
    shift
    if codex_lock_is_held; then
        "$cmd" "$@"
        return $?
    fi
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    if command -v flock >/dev/null 2>&1; then
        (
            if ! flock -w "$CODEX_TERMUX_LOCK_WAIT_SECONDS" -x 9; then
                codex_fail "Another mutation operation is already in progress: $CODEX_TERMUX_LOCK_FILE"
                exit 75
            fi
            export CODEX_TERMUX_LOCK_HELD=1
            "$cmd" "$@"
        ) 9>"$CODEX_TERMUX_LOCK_FILE"
    else
        local lock_dir="${CODEX_TERMUX_LOCK_FILE}.d" waited=0
        while ! mkdir "$lock_dir" 2>/dev/null; do
            sleep 1
            waited=$((waited + 1))
            if [ "$waited" -ge "$CODEX_TERMUX_LOCK_WAIT_SECONDS" ]; then
                codex_fail "Another mutation operation is already in progress: $CODEX_TERMUX_LOCK_FILE"
                return 75
            fi
        done
        (
            trap 'rmdir "$lock_dir" 2>/dev/null || true' EXIT
            export CODEX_TERMUX_LOCK_HELD=1
            "$cmd" "$@"
        )
    fi
}

codex_require_runtime_resolver() {
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "Resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
    fi
}

codex_notify_load_config() {
    local config_file="$CODEX_TERMUX_NOTIFY_CONFIG"
    [ -r "$config_file" ] || return 0
    # shellcheck disable=SC1090
    . "$config_file"
    CODEX_TERMUX_NOTIFY_HOOKS="$(codex_notify_hooks_normalize "${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}")"
}

codex_notify_all_hooks() {
    printf '%s\n' \
        SessionStart \
        PreToolUse \
        PermissionRequest \
        PostToolUse \
        PreCompact \
        PostCompact \
        UserPromptSubmit \
        SubagentStart \
        SubagentStop \
        Stop
}

codex_notify_hook_canonical() {
    case "${1:-}" in
        stop|Stop) printf 'Stop' ;;
        sessionstart|SessionStart) printf 'SessionStart' ;;
        pretooluse|PreToolUse) printf 'PreToolUse' ;;
        permissionrequest|PermissionRequest) printf 'PermissionRequest' ;;
        posttooluse|PostToolUse) printf 'PostToolUse' ;;
        precompact|PreCompact) printf 'PreCompact' ;;
        postcompact|PostCompact) printf 'PostCompact' ;;
        userpromptsubmit|UserPromptSubmit) printf 'UserPromptSubmit' ;;
        subagentstart|SubagentStart) printf 'SubagentStart' ;;
        subagentstop|SubagentStop) printf 'SubagentStop' ;;
        all|ALL) printf 'all' ;;
        *) printf '%s' "${1:-}" ;;
    esac
}

codex_notify_hook_valid() {
    case "$(codex_notify_hook_canonical "${1:-}")" in
        SessionStart|PreToolUse|PermissionRequest|PostToolUse|PreCompact|PostCompact|UserPromptSubmit|SubagentStart|SubagentStop|Stop|all)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

codex_notify_hooks_normalize() {
    local hooks="${1:-Stop}" event seen="," normalized="" event_list=()
    case ",$hooks," in
        *,all,*|*,ALL,*)
            printf 'all\n'
            return 0
            ;;
    esac
    IFS=, read -r -a event_list <<<"$hooks"
    for event in "${event_list[@]}"; do
        event="$(codex_notify_hook_canonical "$event")"
        [ -n "$event" ] || continue
        codex_notify_hook_valid "$event" || continue
        case "$event" in
            all) printf 'all\n'; return 0 ;;
        esac
        case "$seen" in
            *,"$event",*) continue ;;
        esac
        seen="$seen$event,"
        normalized="${normalized:+$normalized,}$event"
    done
    [ -n "$normalized" ] || normalized="Stop"
    printf '%s\n' "$normalized"
}

codex_notify_event_label() {
    case "${1:-}" in
        SessionStart) printf 'session start' ;;
        PreToolUse) printf 'tool start' ;;
        PermissionRequest) printf 'permission request' ;;
        PostToolUse) printf 'tool finished' ;;
        PreCompact) printf 'before compact' ;;
        PostCompact) printf 'after compact' ;;
        UserPromptSubmit) printf 'prompt submitted' ;;
        SubagentStart) printf 'subagent start' ;;
        SubagentStop) printf 'subagent finished' ;;
        Stop) printf 'turn complete' ;;
        *) printf '%s' "${1:-}" ;;
    esac
}

codex_notify_hook_enabled() {
    local event hooks
    event="$(codex_notify_hook_canonical "$1")"
    hooks="$(codex_notify_hooks_normalize "${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}")"
    case ",$hooks," in
        *,all,*) return 0 ;;
        *,"$event",*) return 0 ;;
    esac
    case "$event" in
        PreToolUse)
            [ "$CODEX_TERMUX_NOTIFY_PRETOOLUSE" = "1" ]
            ;;
        *)
            return 1
            ;;
    esac
}

codex_notify_hook_list() {
    local hooks event seen="," event_list=()
    hooks="$(codex_notify_hooks_normalize "${1:-${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}}")"
    case ",$hooks," in
        *,all,*)
            codex_notify_all_hooks
            return 0
            ;;
    esac
    [ -n "$hooks" ] || hooks="Stop"
    IFS=, read -r -a event_list <<<"$hooks"
    for event in "${event_list[@]}"; do
        event="$(codex_notify_hook_canonical "$event")"
        [ -n "$event" ] || continue
        case "$seen" in
            *,"$event",*) continue ;;
        esac
        seen="$seen$event,"
        printf '%s\n' "$event"
    done
}

codex_notify_hook_command() {
    printf '%s --event %s' "$CODEX_TERMUX_TURN_NOTIFY" "$1"
}

codex_notify_hook_status_message() {
    case "$1" in
        SessionStart) printf 'Notify session start' ;;
        PreToolUse) printf 'Notify tool start' ;;
        PermissionRequest) printf 'Notify permission request' ;;
        PostToolUse) printf 'Notify tool finish' ;;
        PreCompact) printf 'Notify before compact' ;;
        PostCompact) printf 'Notify after compact' ;;
        UserPromptSubmit) printf 'Notify prompt submit' ;;
        SubagentStart) printf 'Notify subagent start' ;;
        SubagentStop) printf 'Notify subagent stop' ;;
        Stop) printf 'Notify turn completion' ;;
        *) printf 'Notify %s' "$1" ;;
    esac
}

codex_notify_config_hook_block() {
    local event="$1" command="$2" timeout="${3:-10}" status_message="$4" matcher="${5:-}"
    printf '[[hooks.%s]]\n' "$event"
    [ -n "$matcher" ] && printf 'matcher = "%s"\n' "$matcher"
    printf '\n'
    printf '[[hooks.%s.hooks]]\n' "$event"
    printf 'type = "command"\n'
    printf 'command = "%s"\n' "$command"
    printf 'timeout = %s\n' "$timeout"
    printf 'statusMessage = "%s"\n' "$status_message"
}

codex_notify_write_system_config() {
    local config_file="$CODEX_TERMUX_SYSTEM_CONFIG_DIR/config.toml"
    local event
    mkdir -p "$CODEX_TERMUX_TMPDIR" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR" || return $?
    cat >"$config_file" <<'TOML'
[sandbox_workspace_write]
exclude_slash_tmp = true
exclude_tmpdir_env_var = false
TOML
    codex_notify_load_config
    while IFS= read -r event; do
        codex_notify_hook_enabled "$event" || continue
        codex_notify_config_hook_block \
            "$event" \
            "$(codex_notify_hook_command "$event")" \
            10 \
            "$(codex_notify_hook_status_message "$event")" \
            >>"$config_file"
    done <<EOF
$(codex_notify_hook_list)
EOF
    [ -e "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/requirements.toml" ] ||
        : >"$CODEX_TERMUX_SYSTEM_CONFIG_DIR/requirements.toml"
    [ -e "$CODEX_TERMUX_SYSTEM_CONFIG_DIR/managed_config.toml" ] ||
        : >"$CODEX_TERMUX_SYSTEM_CONFIG_DIR/managed_config.toml"
}

codex_prepare_system_config() {
    codex_notify_write_system_config
}

codex_runtime_exec() {
    local executable="$1"
    shift || true
    local cert_dir_env=() runtime_env=() run_home runtime_dir
    run_home="${CODEX_TERMUX_HOME:-$HOME}"
    runtime_dir="$(codex_parent_dir "$executable")"
    if [ -d "$CODEX_TERMUX_CERT_DIR" ]; then
        cert_dir_env=("SSL_CERT_DIR=$CODEX_TERMUX_CERT_DIR")
    fi
    runtime_env=(env -u LD_PRELOAD -u LD_LIBRARY_PATH \
        -u CODEX_MANAGED_BY_NPM -u CODEX_MANAGED_BY_BUN -u CODEX_MANAGED_PACKAGE_ROOT \
        HOME="$run_home" \
        TMPDIR="$CODEX_TERMUX_TMPDIR" \
        TMP="$CODEX_TERMUX_TMPDIR" \
        TEMP="$CODEX_TERMUX_TMPDIR" \
        SQLITE_TMPDIR="$CODEX_TERMUX_TMPDIR" \
        XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$run_home/.config}" \
        XDG_CACHE_HOME="${XDG_CACHE_HOME:-$run_home/.cache}" \
        XDG_DATA_HOME="${XDG_DATA_HOME:-$run_home/.local/share}" \
        GODEBUG="${GODEBUG:-netdns=go}" \
        SSL_CERT_FILE="$CODEX_TERMUX_CERT_FILE" \
        CODEX_SELF_EXE="$executable" \
        CODEX_TERMUX_BWRAP_COMPAT_QUIET="${CODEX_TERMUX_BWRAP_COMPAT_QUIET:-1}" \
        PATH="$runtime_dir/codex-path:$runtime_dir/codex-resources:$CODEX_TERMUX_PREFIX/bin:$PATH" \
        "${cert_dir_env[@]}")
    codex_require_runtime_resolver || return $?
    codex_prepare_system_config || return $?
    "${runtime_env[@]}" "$executable" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}

codex_smoke_test_runtime() {
    local executable="$1"
    shift || true
    codex_runtime_exec "$executable" --version "$@" >/dev/null 2>&1
}

codex_validate_tarball_safe() {
    codex_termux_cmd validate-tarball --path "$1"
}

codex_replace_tree_atomic() {
    local source="$1" target="$2" backup="$3"
    local existed=0
    codex_assert_managed_tree_target "$source" "replacement source" || return $?
    codex_assert_managed_tree_target "$target" "replacement target" || return $?
    codex_assert_managed_tree_target "$backup" "replacement backup" || return $?
    codex_rm_rf_managed "$backup" || return $?
    if [ -e "$target" ] || [ -L "$target" ]; then
        mv "$target" "$backup" || return 1
        existed=1
    fi
    if mv "$source" "$target"; then
        codex_rm_rf_managed "$backup" || return $?
        return 0
    fi
    if [ "$existed" -eq 1 ]; then
        mv "$backup" "$target" || return 1
    fi
    return 1
}

codex_support_source_dir() {
    if [ -r "$CODEX_TERMUX_MANAGER_DIR/bwrap-termux-compat.py" ] &&
        [ -r "$CODEX_TERMUX_MANAGER_DIR/rg-termux-shim.sh" ]; then
        printf '%s\n' "$CODEX_TERMUX_MANAGER_DIR"
    elif [ -r "$CODEX_TERMUX_RUNTIME_DIR/bwrap-termux-compat.py" ] &&
        [ -r "$CODEX_TERMUX_RUNTIME_DIR/rg-termux-shim.sh" ]; then
        printf '%s\n' "$CODEX_TERMUX_RUNTIME_DIR"
    else
        printf '%s\n' "$CODEX_TERMUX_MANAGER_DIR"
    fi
}

codex_resolve_path() {
    codex_termux_cmd resolve-path --path "$1"
}

codex_tree_digest() {
    codex_termux_cmd tree-digest --path "$1"
}

codex_read_state_field() {
    local field="$1"
    codex_termux_cmd state-read-field \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --field "$field"
}

codex_store_id() {
    local version="$1" sha="$2" tree_sha="${3:-}" builder_sha="unknown" bwrap_sha="unknown" rg_sha="unknown" support_dir
    support_dir="$(codex_support_source_dir)"
    if [ -r "$CODEX_TERMUX_RUNTIME_BUILDER" ]; then
        builder_sha="$(codex_sha256 "$CODEX_TERMUX_RUNTIME_BUILDER")"
    fi
    if [ -r "$support_dir/bwrap-termux-compat.py" ]; then
        bwrap_sha="$(codex_sha256 "$support_dir/bwrap-termux-compat.py")"
    fi
    if [ -r "$support_dir/rg-termux-shim.sh" ]; then
        rg_sha="$(codex_sha256 "$support_dir/rg-termux-shim.sh")"
    fi
    codex_termux_cmd store-id \
        --version "$version" \
        --sha256 "$sha" \
        --builder-sha256 "$builder_sha" \
        --bwrap-sha256 "$bwrap_sha" \
        --rg-sha256 "$rg_sha" \
        --tree-sha256 "$tree_sha"
}

codex_validate_runtime_retention() {
    case "$CODEX_TERMUX_RUNTIME_RETENTION" in
        ''|*[!0-9]*|0)
            codex_fail "CODEX_TERMUX_RUNTIME_RETENTION must be an integer greater than zero"
            return 2
            ;;
    esac
}

codex_prune_runtime_store() {
    local protected_runtime_arg=()
    codex_validate_runtime_retention || return $?
    if [ -n "${CODEX_SELF_EXE:-}" ]; then
        protected_runtime_arg=(--protect-runtime-path "$(codex_parent_dir "$CODEX_SELF_EXE")")
    fi
    codex_termux_cmd store-prune \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_TERMUX_RAW_STORE_DIR" \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --retention "$CODEX_TERMUX_RUNTIME_RETENTION" \
        --current-link "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified-link "$CODEX_TERMUX_VERIFIED_LINK" \
        "${protected_runtime_arg[@]}" \
        --raw-link "$CODEX_TERMUX_RAW_DIR" >/dev/null
}

codex_prepare_complete_runtime_tree() {
    local payload_dir="$1" complete_dir="$2" name support_dir
    support_dir="$(codex_support_source_dir)"
    if { [ ! -r "$support_dir/bwrap-termux-compat.py" ] || [ ! -r "$support_dir/rg-termux-shim.sh" ]; } &&
        [ -r "$payload_dir/bwrap-termux-compat.py" ] &&
        [ -r "$payload_dir/rg-termux-shim.sh" ]; then
        support_dir="$payload_dir"
    fi
    [ -x "$payload_dir/codex" ] || return 1
    codex_rm_rf_managed "$complete_dir" || return $?
    mkdir -p "$complete_dir"
    for name in codex codex-resources codex-path codex-package.json runtime-build.json; do
        [ -e "$payload_dir/$name" ] || {
            codex_rm_rf_managed "$complete_dir" || return $?
            return 1
        }
        cp -R "$payload_dir/$name" "$complete_dir/$name"
    done
    [ -x "$CODEX_TERMUX_RUNTIME_BUILDER" ] &&
        [ -r "$support_dir/bwrap-termux-compat.py" ] &&
        [ -r "$support_dir/rg-termux-shim.sh" ] || {
        codex_rm_rf_managed "$complete_dir" || return $?
        return 1
    }
    cp -R "$support_dir/bwrap-termux-compat.py" "$complete_dir/codex-path/bwrap"
    cp -R "$support_dir/rg-termux-shim.sh" "$complete_dir/codex-path/rg"
    rm -f "$complete_dir/codex-resources/bwrap.real"
    chmod 755 "$complete_dir/codex" \
        "$complete_dir/codex-path/bwrap" \
        "$complete_dir/codex-path/rg" \
        "$complete_dir/codex-path/rg.real" 2>/dev/null || true
}

codex_package_spec() {
    local requested="${1:-}"
    if [ -z "$requested" ] || [ "$requested" = "latest" ] || [ "$requested" = "stable" ]; then
        printf '%s\n' "$CODEX_TERMUX_PACKAGE_SPEC_DEFAULT"
    elif [[ "$requested" == @openai/codex@* ]]; then
        printf '%s\n' "$requested"
    elif [[ "$requested" == *linux-arm64 ]]; then
        printf '@openai/codex@%s\n' "$requested"
    else
        printf '@openai/codex@%s-linux-arm64\n' "$requested"
    fi
}

codex_extract_pack_field() {
    codex_termux_cmd package-field --json-file "$1" --field "$2"
}

codex_fetch_package() {
    local requested="${1:-}" package_spec tmp pack_json tgz filename version
    package_spec="$(codex_package_spec "$requested")"
    tmp="$(codex_mktemp_dir codex-pack)" || return 1
    pack_json="$tmp/pack.json"
    codex_ui_step fetch_package "$package_spec"
    if ! npm pack "$package_spec" --json --pack-destination "$tmp" >"$pack_json"; then
        rm -rf "$tmp"
        codex_fail "Failed to fetch $package_spec"
        return 1
    fi
    filename="$(codex_extract_pack_field "$pack_json" filename)"
    version="$(codex_extract_pack_field "$pack_json" version)"
    tgz="$tmp/$filename"
    if [ ! -f "$tgz" ]; then
        rm -rf "$tmp"
        codex_fail "Package fetch did not produce the expected tarball"
        return 1
    fi
    codex_ui_step validate_archive
    mkdir -p "$tmp/package"
    if ! codex_validate_tarball_safe "$tgz" >/dev/null 2>&1; then
        rm -rf "$tmp"
        codex_fail "Package archive contains unsafe paths: $tgz"
        return 1
    fi
    codex_ui_step unpack_archive
    if ! tar -xzf "$tgz" -C "$tmp/package" --strip-components=1; then
        rm -rf "$tmp"
        codex_fail "Failed to extract $tgz"
        return 1
    fi
    printf '%s\t%s\t%s\t%s\n' "$tmp" "$tmp/package/vendor/aarch64-unknown-linux-musl" "$version" "$package_spec"
}

codex_install_raw_vendor() {
    local src_vendor="$1" target_dir="${2:-$CODEX_TERMUX_RAW_DIR}" staged
    staged="$target_dir.new.$$"
    codex_rm_rf_managed "$staged" || return $?
    mkdir -p "$staged/vendor"
    cp -R "$src_vendor" "$staged/vendor/aarch64-unknown-linux-musl"
    chmod 755 "$staged/vendor/aarch64-unknown-linux-musl/bin/codex"
    if ! codex_replace_tree_atomic "$staged" "$target_dir" "$target_dir.old"; then
        codex_rm_rf_managed "$staged" || return $?
        return 1
    fi
}

codex_build_runtime_tree() {
    local raw_vendor="$1"
    local runtime_dir="$2"
    local log_file="$3"
    local builder="$CODEX_TERMUX_RUNTIME_BUILDER"
    local report_file="${log_file}.report.json"

    mkdir -p "$(codex_parent_dir "$runtime_dir")"
    if ! python3 "$builder" "$raw_vendor" --runtime-dir "$runtime_dir" --report-json "$report_file" >"$log_file" 2>&1; then
        return 70
    fi
    if ! codex_smoke_test_runtime "$runtime_dir/codex"; then
        return 72
    fi
    return 0
}

codex_activate_tuple_unlocked() {
    local runtime_src="$1" version="$2" raw_sha="$3" runtime_sha="$4" package_spec="$5" raw_src="${6:-}"
    local raw_store_src="$CODEX_TERMUX_RAW_DIR" runtime_target raw_target
    local cleanup_raw=() runtime_tree_sha raw_tree_sha
    if [ -n "$raw_src" ]; then
        raw_store_src="$raw_src"
        cleanup_raw=(--cleanup-raw-source)
    fi
    runtime_tree_sha="$(codex_tree_digest "$(codex_resolve_path "$runtime_src")")"
    raw_tree_sha="$(codex_tree_digest "$(codex_resolve_path "$raw_store_src")")"
    runtime_target="$CODEX_TERMUX_RUNTIME_STORE_DIR/$(codex_store_id "$version" "$runtime_sha" "$runtime_tree_sha")"
    raw_target="$CODEX_TERMUX_RAW_STORE_DIR/$(codex_store_id "$version" "$raw_sha" "$raw_tree_sha")"
    codex_termux_activation_cmd activation-commit \
        --candidate-runtime "$runtime_src" \
        --candidate-raw "$raw_store_src" \
        --runtime-target "$runtime_target" \
        --raw-target "$raw_target" \
        --version "$version" \
        --raw-sha256 "$raw_sha" \
        --runtime-sha256 "$runtime_sha" \
        --package-spec "$package_spec" \
        --cleanup-runtime-source \
        "${cleanup_raw[@]}" >/dev/null || return 1
    codex_prune_runtime_store
}

codex_commit_runtime_candidate() {
    codex_activate_tuple_unlocked "$@"
}

codex_runtime_build_cached_unlocked() {
    local version="${1:-unknown}" package_spec="${2:-local}" report build_stdout raw_sha runtime_sha
    local runtime_stage="$CODEX_TERMUX_RUNTIME_DIR.build.$$" runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.complete.$$"
    [ -x "$CODEX_TERMUX_RAW_VENDOR/bin/codex" ] || return 1
    mkdir -p "$CODEX_TERMUX_STATE_DIR" "$CODEX_TERMUX_DOCTOR_DIR"
    report="$CODEX_TERMUX_DOCTOR_DIR/last-build-report.json"
    build_stdout="$CODEX_TERMUX_DOCTOR_DIR/last-build-report.stdout"
    codex_rm_rf_managed "$runtime_stage" || return $?
    if [ "${CODEX_TERMUX_BUILD_VERBOSE:-0}" = "1" ]; then
        if ! "$CODEX_TERMUX_RUNTIME_BUILDER" "$CODEX_TERMUX_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report"; then
            codex_rm_rf_managed "$runtime_stage" || return $?
            return 1
        fi
    else
        if ! "$CODEX_TERMUX_RUNTIME_BUILDER" "$CODEX_TERMUX_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report" >"$build_stdout" 2>&1; then
            codex_rm_rf_managed "$runtime_stage" || return $?
            return 1
        fi
    fi
    codex_ui_step assemble_runtime
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        codex_rm_rf_managed "$runtime_stage" "$runtime_complete" || return $?
        return 1
    fi
    raw_sha="$(codex_sha256 "$CODEX_TERMUX_RAW_VENDOR/bin/codex")"
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    codex_rm_rf_managed "$runtime_stage" || return $?
    codex_ui_step smoke_test_runtime
    if ! codex_smoke_test_runtime "$runtime_complete/codex"; then
        codex_rm_rf_managed "$runtime_complete" || return $?
        return 1
    fi
    codex_ui_step activate_runtime
    codex_commit_runtime_candidate "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec"
}

codex_runtime_install_cached_unlocked() {
    local version package_spec
    codex_raw_integrity_ok || {
        codex_fail "Cached raw package integrity check failed; run codex update"
        return 1
    }
    version="$(codex_read_state_field version)"
    package_spec="$(codex_read_state_field package_spec)"
    [ -n "$version" ] || version="unknown"
    [ -n "$package_spec" ] || package_spec="local"
    codex_runtime_build_cached_unlocked "$version" "$package_spec" || return $?
}

codex_runtime_install_cached() {
    local status=0
    codex_with_lock codex_runtime_install_cached_unlocked || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

CODEX_REPAIR_NEEDS_SUPPORT=0
CODEX_REPAIR_NEEDS_RUNTIME=0
CODEX_REPAIR_NEEDS_METADATA=0
CODEX_REPAIR_RAW_OK=0

codex_repair_diagnose() {
    CODEX_REPAIR_NEEDS_SUPPORT=0
    CODEX_REPAIR_NEEDS_RUNTIME=0
    CODEX_REPAIR_NEEDS_METADATA=0
    CODEX_REPAIR_RAW_OK=0

    codex_support_layer_ok || CODEX_REPAIR_NEEDS_SUPPORT=1
    codex_raw_integrity_ok && CODEX_REPAIR_RAW_OK=1
    if codex_runtime_ok; then
        codex_runtime_metadata_current || CODEX_REPAIR_NEEDS_METADATA=1
    else
        CODEX_REPAIR_NEEDS_RUNTIME=1
    fi
}

codex_repair_install_support() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_ui_step repair_support
    CODEX_TERMUX_INSTALL_PRINT_VERSION=0 CODEX_TERMUX_INSTALL_SURFACE=0 bash "$source" install support >/dev/null
}

codex_repair_apply() {
    if [ "$CODEX_REPAIR_NEEDS_SUPPORT" = "1" ]; then
        codex_repair_install_support || return $?
    fi

    if codex_runtime_ok; then
        if ! codex_runtime_metadata_current; then
            codex_ui_step repair_metadata
            codex_refresh_runtime_metadata || return $?
        fi
        return 0
    fi

    if codex_try_verified_rollback; then
        codex_refresh_runtime_metadata || return $?
        return 0
    fi

    codex_raw_integrity_ok || {
        codex_fail "Runtime is damaged and cached raw is unavailable or invalid; run codex update"
        return 1
    }
    codex_ui_step repair_runtime
    codex_runtime_install_cached || return $?
    codex_refresh_runtime_metadata
}

codex_repair_core_unlocked() {
    codex_validate_runtime_retention || return $?
    codex_repair_diagnose
    codex_repair_apply
}

codex_repair_public_unlocked() {
    codex_repair_core_unlocked || return $?
    codex_version
}

codex_repair_public() {
    local status=0
    codex_with_lock codex_repair_public_unlocked || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

codex_runtime_install_upstream_unlocked() {
    local requested="${1:-}" fetched tmp vendor version spec raw_stage runtime_stage runtime_complete raw_sha runtime_sha
    fetched="$(codex_fetch_package "$requested")" || return $?
    IFS=$'\t' read -r tmp vendor version spec <<EOF
$fetched
EOF
    raw_stage="$CODEX_TERMUX_RAW_DIR.update.$$"
    runtime_stage="$CODEX_TERMUX_RUNTIME_DIR.update.$$"
    runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.complete.$$"
    mkdir -p "$CODEX_TERMUX_DOCTOR_DIR"
    codex_ui_step stage_raw
    if ! codex_install_raw_vendor "$vendor" "$raw_stage"; then
        rm -rf "$tmp"
        return 1
    fi
    codex_ui_step build_runtime
    if ! codex_build_runtime_tree "$raw_stage/vendor/aarch64-unknown-linux-musl" "$runtime_stage" "$CODEX_TERMUX_DOCTOR_DIR/last-build-report.stdout"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_stage" "$runtime_complete" || return $?
        return 1
    fi
    codex_ui_step assemble_runtime
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_stage" "$runtime_complete" || return $?
        return 1
    fi
    raw_sha="$(codex_sha256 "$raw_stage/vendor/aarch64-unknown-linux-musl/bin/codex")"
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    codex_rm_rf_managed "$runtime_stage" || return $?
    codex_ui_step smoke_test_runtime
    if ! codex_smoke_test_runtime "$runtime_complete/codex"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_complete" || return $?
        return 1
    fi
    codex_ui_step activate_runtime
    if ! codex_commit_runtime_candidate "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$spec" "$raw_stage"; then
        rm -rf "$tmp"
        codex_rm_rf_managed "$raw_stage" "$runtime_complete" || return $?
        return 1
    fi
    rm -rf "$tmp"
    codex_ui_step switch_runtime "$(codex_display_version "$version")"
}

codex_runtime_install_upstream() {
    local status=0
    codex_validate_runtime_retention || return $?
    codex_with_lock codex_runtime_install_upstream_unlocked "${1:-}" || status=$?
    [ "$status" -eq 0 ] || codex_status_clear
    return "$status"
}

codex_latest_linux_arm64_version() {
    if command -v timeout >/dev/null 2>&1; then
        timeout "$CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | tr -d '"'
    else
        npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | tr -d '"'
    fi
}

codex_auto_update_mode() {
    local mode="${CODEX_TERMUX_AUTO_UPDATE_MODE:-prompt}"
    case "$mode" in
        0|off|false|no|none)
            printf 'off\n'
            ;;
        force|auto|always)
            printf 'force\n'
            ;;
        1|prompt|ask|"")
            printf 'prompt\n'
            ;;
        *)
            printf 'prompt\n'
            ;;
    esac
}

codex_auto_update_due() {
    local now last
    [ "$CODEX_TERMUX_AUTO_UPDATE" = "0" ] && return 1
    [ "$(codex_auto_update_mode)" != "off" ] || return 1
    now="$(date +%s)"
    last="$(cat "$CODEX_TERMUX_AUTO_UPDATE_STAMP" 2>/dev/null || printf '0')"
    [ $((now - last)) -ge "$CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS" ]
}

codex_mark_auto_update_checked() {
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    date +%s >"$CODEX_TERMUX_AUTO_UPDATE_STAMP"
}

codex_read_pending_auto_update() {
    cat "$CODEX_TERMUX_AUTO_UPDATE_PENDING" 2>/dev/null || true
}

codex_write_pending_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\n' "$version" >"$CODEX_TERMUX_AUTO_UPDATE_PENDING"
}

codex_clear_pending_auto_update() {
    rm -f "$CODEX_TERMUX_AUTO_UPDATE_PENDING"
}

codex_read_failed_auto_update() {
    cat "$CODEX_TERMUX_AUTO_UPDATE_FAILED" 2>/dev/null || true
}

codex_write_failed_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\t%s\n' "$version" "$(date +%s)" >"$CODEX_TERMUX_AUTO_UPDATE_FAILED"
}

codex_clear_failed_auto_update() {
    rm -f "$CODEX_TERMUX_AUTO_UPDATE_FAILED"
}

codex_failed_auto_update_due() {
    local version="$1" failed failed_version failed_at now
    failed="$(codex_read_failed_auto_update)"
    [ -n "$failed" ] || return 0
    IFS=$'\t' read -r failed_version failed_at <<EOF
$failed
EOF
    [ "$failed_version" = "$version" ] || return 0
    case "$failed_at" in
        ''|*[!0-9]*) return 0 ;;
    esac
    now="$(date +%s)"
    [ $((now - failed_at)) -ge "$CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS" ]
}

codex_prompt_update() {
    local current="$1" latest="$2" choice display_current display_latest
    [ -t 0 ] && [ -t 2 ] || return 1
    display_current="$(codex_display_version "$current")"
    display_latest="$(codex_display_version "$latest")"
    codex_confirm_menu \
        "$(codex_ui_text_get update_available_title)" \
        "$display_current -> $display_latest" \
        y "$display_latest" "$(codex_ui_badge update)" \
        N "$display_current" "$(codex_ui_badge current) $(codex_ui_badge keep)" \
        "$(codex_ui_text_get apply_update_prompt)" \
        cancel || {
        [ "$?" -eq 130 ] && return 130
        return 1
    }
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    case "$choice" in
        y|Y)
            return 0
            ;;
        n|N)
            codex_say "$(codex_ui_text_get current_kept "$(codex_display_version "$current")")"
            return 1
            ;;
        *)
            return 130
            ;;
    esac
}

codex_install_auto_update() {
    local current="$1" latest="$2"
    codex_ui_step update_runtime "$current" "$latest"
    if codex_runtime_install_upstream "$latest"; then
        codex_clear_pending_auto_update
        codex_clear_failed_auto_update
    else
        codex_write_failed_auto_update "$latest"
        codex_say "$(codex_ui_text_get update_failed_continue "$current")"
        return 1
    fi
}

codex_auto_update_if_needed() {
    local current latest mode pending
    codex_runtime_ok || return 0
    [ "$CODEX_TERMUX_AUTO_UPDATE" = "0" ] && return 0
    [ "$(codex_auto_update_mode)" != "off" ] || return 0
    current="$(codex_read_state_field version)"
    pending="$(codex_read_pending_auto_update)"
    if [ -n "$pending" ] && [ "$pending" = "$current" ]; then
        codex_clear_pending_auto_update
        pending=""
    fi
    if [ -n "$pending" ] && [ "$pending" != "$current" ] && ! codex_auto_update_due; then
        latest="$pending"
    else
        codex_auto_update_due || return 0
        codex_mark_auto_update_checked
        latest="$(codex_latest_linux_arm64_version || true)"
        if [ -z "$latest" ]; then
            [ -z "$pending" ] || codex_clear_pending_auto_update
            return 0
        fi
    fi
    if [ "$latest" != "$current" ]; then
        codex_write_pending_auto_update "$latest"
        codex_failed_auto_update_due "$latest" || return 0
        mode="$(codex_auto_update_mode)"
        if [ "$mode" = "force" ]; then
            codex_install_auto_update "$current" "$latest" || return 0
        else
            codex_prompt_update "$current" "$latest"
            case "$?" in
                0)
                    codex_install_auto_update "$current" "$latest" || return 0
                    ;;
                130)
                    return 130
                    ;;
            esac
        fi
    else
        codex_clear_pending_auto_update
    fi
    return 0
}


# Runtime readiness and diagnostics.
codex_support_tools_match() {
    local support_dir
    support_dir="$(codex_support_source_dir)"
    cmp -s "$support_dir/bwrap-termux-compat.py" "$CODEX_TERMUX_RUNTIME_DIR/codex-path/bwrap" &&
    cmp -s "$support_dir/rg-termux-shim.sh" "$CODEX_TERMUX_RUNTIME_DIR/codex-path/rg"
}

codex_runtime_integrity_ok() {
    [ -r "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" ] || return 1
    [ -x "$CODEX_TERMUX_RUNTIME_BUILDER" ] || return 1
    codex_termux_cmd runtime-integrity \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --manifest-path "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" \
        --builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY"
}

codex_raw_integrity_ok() {
    [ -x "$CODEX_TERMUX_RAW_VENDOR/bin/codex" ] || return 1
    codex_termux_cmd raw-integrity \
        --raw-binary "$CODEX_TERMUX_RAW_VENDOR/bin/codex" \
        --state-path "$CODEX_TERMUX_STATE_FILE"
}

codex_runtime_ok() {
    [ -x "$CODEX_TERMUX_RUNTIME" ] &&
    [ -x "$CODEX_TERMUX_RUNTIME_DIR/codex-resources/bwrap" ] &&
    [ -x "$CODEX_TERMUX_RUNTIME_DIR/codex-path/bwrap" ] &&
    [ -x "$CODEX_TERMUX_RUNTIME_DIR/codex-path/rg" ] &&
    [ -x "$CODEX_TERMUX_RUNTIME_DIR/codex-path/rg.real" ] &&
    codex_support_tools_match &&
    [ -r "$CODEX_TERMUX_STATE_FILE" ] &&
    codex_runtime_integrity_ok
}

codex_support_layer_ok() {
    [ -x "$CODEX_TERMUX_MANAGED_SHELL" ] &&
    [ -r "$CODEX_TERMUX_MANAGER_DIR/lib.sh" ] &&
    [ -x "$CODEX_TERMUX_MANAGER_DIR/build-runtime.py" ] &&
    [ -x "$CODEX_TERMUX_MANAGER_DIR/bwrap-termux-compat.py" ] &&
    [ -x "$CODEX_TERMUX_MANAGER_DIR/rg-termux-shim.sh" ] &&
    [ -x "$CODEX_TERMUX_MANAGER_DIR/codex-turn-notify.sh" ] &&
    [ -e "$CODEX_TERMUX_PUBLIC_CODEX" ] &&
    codex_file_has_marker "$CODEX_TERMUX_PUBLIC_CODEX"
}

codex_runtime_metadata_current() {
    local wrapper_version wrapper_commit
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    codex_termux_cmd runtime-metadata-current \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --wrapper-version "$wrapper_version" \
        --wrapper-commit "$wrapper_commit"
}

codex_refresh_runtime_metadata_unlocked() {
    local version raw_sha runtime_sha package_spec
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    package_spec="$(codex_read_state_field package_spec)"
    [ -n "$version" ] && [ -n "$raw_sha" ] && [ -n "$runtime_sha" ] && [ -n "$package_spec" ] || return 0
    codex_runtime_metadata_current && return 0
    codex_activate_tuple_unlocked \
        "$CODEX_TERMUX_RUNTIME_DIR" "$version" "$raw_sha" "$runtime_sha" "$package_spec"
}

codex_refresh_runtime_metadata() {
    codex_runtime_metadata_current && return 0
    codex_with_lock codex_refresh_runtime_metadata_unlocked
}

codex_activate_cached_runtime_unlocked() {
    local runtime_path="$1" raw_path="$2" version="$3" raw_sha="$4" runtime_sha="$5" package_spec="$6"
    local runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.use.$$" raw_complete="$CODEX_TERMUX_RAW_DIR.use.$$"
    codex_prepare_complete_runtime_tree "$runtime_path" "$runtime_complete" || return 1
    if ! codex_install_raw_vendor "$raw_path/vendor/aarch64-unknown-linux-musl" "$raw_complete"; then
        codex_rm_rf_managed "$runtime_complete" "$raw_complete" || return $?
        return 1
    fi
    if ! codex_commit_runtime_candidate "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$raw_complete"; then
        codex_rm_rf_managed "$runtime_complete" "$raw_complete" || return $?
        return 1
    fi
    codex_ui_step switch_runtime "$version"
}


codex_try_verified_rollback_unlocked() {
    codex_termux_activation_cmd activation-restore-verified >/dev/null || return 1
    codex_say "$(codex_ui_text_get restored_verified)"
}

codex_verified_rollback_needed() {
    [ -e "$CODEX_TERMUX_VERIFIED_LINK" ] || [ -L "$CODEX_TERMUX_VERIFIED_LINK" ] || return 1
    [ -x "$(codex_resolve_path "$CODEX_TERMUX_VERIFIED_LINK")/codex" ] || return 1
    [ "$(codex_resolve_path "$CODEX_TERMUX_RUNTIME_DIR")" != "$(codex_resolve_path "$CODEX_TERMUX_VERIFIED_LINK")" ]
}

codex_try_verified_rollback() {
    codex_verified_rollback_needed || return 1
    codex_with_lock codex_try_verified_rollback_unlocked
}


codex_ensure_runtime_ready() {
    if codex_runtime_ok; then
        codex_refresh_runtime_metadata
        return 0
    fi
    if codex_try_verified_rollback; then
        codex_refresh_runtime_metadata
        return 0
    fi
    if [ -x "$CODEX_TERMUX_RAW_VENDOR/bin/codex" ]; then
        if ! codex_raw_integrity_ok; then
            codex_fail "Cached raw package integrity check failed; run codex update"
            return 1
        fi
        codex_ui_step rebuild_cached_runtime
        codex_runtime_install_cached
        return $?
    fi
    codex_fail "Runtime is missing and no cached raw package is available; run codex update"
    return 127
}


codex_prepare_runtime_env() {
    local runtime_dir runtime_exe
    codex_prepare_system_config || return $?
    runtime_dir="$(codex_resolve_path "$CODEX_TERMUX_RUNTIME_DIR")" || return $?
    runtime_exe="$runtime_dir/codex"
    export TMPDIR="$CODEX_TERMUX_TMPDIR"
    export TMP="$CODEX_TERMUX_TMPDIR"
    export TEMP="$CODEX_TERMUX_TMPDIR"
    export SQLITE_TMPDIR="$CODEX_TERMUX_TMPDIR"
    export SSL_CERT_FILE="${SSL_CERT_FILE:-$CODEX_TERMUX_CERT_FILE}"
    [ -d "$CODEX_TERMUX_CERT_DIR" ] && export SSL_CERT_DIR="${SSL_CERT_DIR:-$CODEX_TERMUX_CERT_DIR}"
    if [ -z "${BROWSER:-}" ] && command -v termux-open-url >/dev/null 2>&1; then
        export BROWSER=termux-open-url
    fi
    export CODEX_SELF_EXE="$runtime_exe"
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    export CODEX_TERMUX_BWRAP_COMPAT_QUIET="${CODEX_TERMUX_BWRAP_COMPAT_QUIET:-1}"
    export PATH="$runtime_dir/codex-path:$runtime_dir/codex-resources:$CODEX_TERMUX_PREFIX/bin:$PATH"
}

codex_run_current_runtime() {
    codex_status_clear
    codex_prepare_runtime_env || return $?
    codex_require_runtime_resolver || return $?
    "$CODEX_SELF_EXE" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}

codex_exec_current_runtime() {
    codex_status_clear
    codex_prepare_runtime_env || return $?
    codex_require_runtime_resolver || return $?
    exec "$CODEX_SELF_EXE" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
}

codex_current_wrapper_version() {
    if [ -f "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    elif [ -f "$CODEX_TERMUX_RUNTIME_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_TERMUX_RUNTIME_DIR/wrapper-version.env"
    fi
    printf '%s\n' "${CODEX_TERMUX_WRAPPER_VERSION:-unknown}"
}

codex_current_wrapper_commit() {
    if [ -f "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_TERMUX_MANAGER_DIR/wrapper-version.env"
    elif [ -f "$CODEX_TERMUX_RUNTIME_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_TERMUX_RUNTIME_DIR/wrapper-version.env"
    fi
    printf '%s\n' "${CODEX_TERMUX_WRAPPER_COMMIT:-unknown}"
}

codex_current_runtime_date() {
    codex_termux_cmd registry-active-runtime-date \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE"
}

codex_display_dotted_date() {
    local value="${1:-}" digits
    value="${value%%T*}"
    case "$value" in
        ????-??-??*)
            printf '%s\n' "${value%%T*}"
            ;;
        [0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]*)
            printf '%s-%s-%s\n' "${value:0:4}" "${value:4:2}" "${value:6:2}"
            ;;
        *)
            digits="${value//[^0-9]/}"
            if [ "${#digits}" -ge 8 ]; then
                printf '%s-%s-%s\n' "${digits:0:4}" "${digits:4:2}" "${digits:6:2}"
            else
                printf '%s\n' "$value"
            fi
            ;;
    esac
}

codex_read_upstream_release_date_cache() {
    local version="$1"
    [ -r "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" ] || return 1
    awk -F '\t' -v version="$version" '$1 == version { print $2; found=1; exit } END { exit found ? 0 : 1 }' \
        "$CODEX_TERMUX_UPSTREAM_TIME_CACHE"
}

codex_write_upstream_release_date_cache() {
    local version="$1" release_date="$2" tmp
    [ -n "$version" ] && [ -n "$release_date" ] || return 0
    mkdir -p "$(codex_parent_dir "$CODEX_TERMUX_UPSTREAM_TIME_CACHE")" 2>/dev/null || return 0
    tmp="$CODEX_TERMUX_UPSTREAM_TIME_CACHE.$$"
    if [ -r "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" ]; then
        awk -F '\t' -v version="$version" '$1 != version' "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" >"$tmp" 2>/dev/null || : >"$tmp"
    else
        : >"$tmp"
    fi
    printf '%s\t%s\n' "$version" "$release_date" >>"$tmp"
    mv -f "$tmp" "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" 2>/dev/null || rm -f "$tmp"
}

codex_fetch_upstream_release_date() {
    local version="${1:-}"
    [ -n "$version" ] || return 0
    if command -v timeout >/dev/null 2>&1; then
        timeout "$CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex time --json 2>/dev/null | \
            python3 -c '
import json
import re
import sys

version = sys.argv[1]
payload = sys.stdin.read()
try:
    data = json.loads(payload)
except Exception:
    sys.exit(0)
value = data.get(version, "")
if not value:
    sys.exit(0)
text = str(value).split("T", 1)[0]
match = re.match(r"(\d{4})[-.](\d{2})[-.](\d{2})", text)
if match:
    print("-".join(match.groups()))
else:
    digits = "".join(ch for ch in text if ch.isdigit())
    if len(digits) >= 8:
        print(f"{digits[:4]}-{digits[4:6]}-{digits[6:8]}")
' "$version"
    else
        npm view @openai/codex time --json 2>/dev/null | \
            python3 -c '
import json
import re
import sys

version = sys.argv[1]
payload = sys.stdin.read()
try:
    data = json.loads(payload)
except Exception:
    sys.exit(0)
value = data.get(version, "")
if not value:
    sys.exit(0)
text = str(value).split("T", 1)[0]
match = re.match(r"(\d{4})[-.](\d{2})[-.](\d{2})", text)
if match:
    print("-".join(match.groups()))
else:
    digits = "".join(ch for ch in text if ch.isdigit())
    if len(digits) >= 8:
        print(f"{digits[:4]}-{digits[4:6]}-{digits[6:8]}")
	' "$version"
    fi
}

codex_upstream_release_date() {
    local version="${1:-}" release_date
    [ -n "$version" ] || return 0
    release_date="$(codex_read_upstream_release_date_cache "$version" 2>/dev/null || true)"
    if [ -n "$release_date" ]; then
        printf '%s\n' "$release_date"
        return 0
    fi
    release_date="$(codex_fetch_upstream_release_date "$version" || true)"
    if [ -n "$release_date" ]; then
        codex_write_upstream_release_date_cache "$version" "$release_date"
        printf '%s\n' "$release_date"
    fi
}

codex_version() {
    local upstream upstream_version upstream_date runtime_date wrapper_version wrapper_commit status=0
    codex_status_clear
    if upstream="$(codex_run_current_runtime --version 2>/dev/null)"; then
        status=0
    else
        status=$?
        upstream=""
    fi
    upstream_version="${upstream#codex-cli }"
    [ -n "$upstream_version" ] || upstream_version="unknown"
    upstream_date="$(codex_upstream_release_date "$upstream_version" || true)"
    runtime_date="$(codex_display_dotted_date "$(codex_current_runtime_date || true)")"
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    printf '%s' "$upstream" >&2
    [ -n "$upstream_date" ] && printf ' (%s)' "$upstream_date" >&2
    printf '\n' >&2
    printf '%-9s %s\n' 'runtime' "${runtime_date:-unknown}" >&2
    printf '%-9s %s' 'wrapper' "$wrapper_version" >&2
    [ -n "$wrapper_commit" ] && [ "$wrapper_commit" != "unknown" ] && printf ' (%s)' "$wrapper_commit" >&2
    printf '\n' >&2
    return "$status"
}

codex_wrapper_help() {
    codex_status_clear
    printf '\n'
    printf 'Wrapper commands\n'
    printf '  %-8s  %s\n' 'codex' 'Managed upstream Codex entrypoint; bare execution may auto-update before launch.'
    printf '  %-8s  %s\n' 'install' 'Install all components, or use support/upstream/rebuild modes.'
    printf '  %-8s  %s\n' 'repair' 'Diagnose and repair the managed installation.'
    printf '  %-8s  %s\n' 'notify' 'Configure notification/toast channels and regenerate hook configuration.'
    printf '  %-8s  %s\n' 'update' 'Refresh wrapper support and install a fresh patched runtime.'
    printf '  %-8s  %s\n' 'use' 'List cached and remote runtimes; promote the selected runtime.'
    printf '  %-8s  %s\n' 'session' 'Resume previous Codex sessions across profiles.'
    printf '  %-8s  %s\n' 'profile' 'List numbered profiles or enter a named profile with CODEX_HOME switched.'
    printf '  %-8s  %s\n' 'doctor' 'Check launcher, runtime resources, resolver, CA, DNS patch, and state.'
    printf '  %-8s  %s\n' 'version' 'Print upstream and runtime version/date rows.'
    printf '  %-8s  %s\n' 'remove' 'Remove managed launcher/runtime and restore launcher backups when present.'
}

codex_help() {
    if [ -x "$CODEX_TERMUX_RUNTIME" ]; then
        codex_run_current_runtime --help
    fi
    codex_wrapper_help
}


codex_wrapper_doctor_json() {
    local version raw_sha runtime_sha
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    codex_prepare_runtime_env
    codex_termux_cmd doctor-report \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --current-link "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified-link "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw-link "$CODEX_TERMUX_RAW_DIR" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_TERMUX_RAW_STORE_DIR" \
        --raw-vendor "$CODEX_TERMUX_RAW_VENDOR" \
        --resolv-conf "$CODEX_TERMUX_RESOLV_CONF" \
        --cert-file "$CODEX_TERMUX_CERT_FILE" \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --version "$version" \
        --raw-sha256 "$raw_sha" \
        --runtime-sha256 "$runtime_sha" \
        --prefix "$CODEX_TERMUX_PREFIX" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY"
}

codex_wrapper_doctor() {
    codex_status_clear
    if [ "${1:-}" = "--json" ]; then
        codex_wrapper_doctor_json
    else
        codex_wrapper_doctor_json | codex_termux_cmd doctor-render --mode human
    fi
}

codex_public_doctor() {
    codex_status_clear
    if [ $# -gt 0 ]; then
        codex_ensure_runtime_ready || return $?
        codex_run_current_runtime doctor "$@"
        return $?
    fi
    local upstream_status=0 wrapper_status=0
    codex_ensure_runtime_ready || return $?
    codex_run_current_runtime doctor || upstream_status=$?
    printf '\n' >&2
    codex_ui_separator
    printf '\n' >&2
    codex_wrapper_doctor || wrapper_status=$?
    [ "$upstream_status" -eq 0 ] && [ "$wrapper_status" -eq 0 ]
}

# Interactive wrapper commands.
codex_profile_validate_name() {
    local profile="${1:-}"
    case "$profile" in
        ""|default)
            return 0
            ;;
        termux|-*|.*|*/*|*..*|*[[:space:]]*)
            return 1
            ;;
        *)
            return 0
            ;;
    esac
}

codex_profile_dir() {
    local profile="${1:-default}"
    if [ -z "$profile" ] || [ "$profile" = "default" ]; then
        printf '%s\n' "$CODEX_TERMUX_HOME/.codex"
    else
        printf '%s/%s\n' "$CODEX_TERMUX_PROFILE_ROOT" "$profile"
    fi
}

codex_profile_display_name() {
    local profile="${1:-default}"
    if [ -z "$profile" ] || [ "$profile" = "default" ]; then
        printf 'default\n'
    else
        printf '%s\n' "$profile"
    fi
}

codex_profile_is_default() {
    local profile="${1:-default}"
    [ -z "$profile" ] || [ "$profile" = "default" ]
}

codex_profile_choice_to_name() {
    local choice="${1:-}"
    case "$choice" in
        ""|home|default)
            printf 'default\n'
            ;;
        *)
            printf '%s\n' "$choice"
            ;;
    esac
}

codex_profile_write_recent() {
    local profile="${1:-default}"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\n' "$profile" >"$CODEX_TERMUX_LAST_PROFILE_FILE"
}

codex_profile_read_recent() {
    local profile
    profile="$(cat "$CODEX_TERMUX_LAST_PROFILE_FILE" 2>/dev/null || true)"
    profile="$(codex_profile_choice_to_name "$profile")"
    codex_profile_validate_name "$profile" || {
        printf 'default\n'
        return 0
    }
    if [ "$profile" != "default" ] && [ ! -d "$(codex_profile_dir "$profile")" ]; then
        printf 'default\n'
        return 0
    fi
    printf '%s\n' "$profile"
}

codex_profile_note() {
    local profile="${1:-default}"
    codex_ui_step open_profile "$(codex_profile_display_name "$profile")"
}

codex_profile_runtime_exec() {
    local profile="$1" profile_dir="$2"
    shift 2 || true
    codex_profile_write_recent "$profile"
    codex_profile_note "$profile"
    if ! codex_profile_is_default "$profile"; then
        export CODEX_HOME="$profile_dir"
    fi
    codex_exec_current_runtime "$@"
}

codex_profile_menu_ids() {
    local profile
    printf 'default\n'
    while IFS= read -r profile; do
        [ "$profile" = "default" ] && continue
        printf '%s\n' "$profile"
    done < <(codex_list_profiles)
}

codex_list_profiles() {
    local root="$CODEX_TERMUX_PROFILE_ROOT" profile
    [ -d "$root" ] || return 0
    while IFS= read -r profile; do
        codex_profile_validate_name "$profile" || continue
        [ "$profile" = "default" ] && continue
        printf '%s\n' "$profile"
    done < <(find "$root" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null) \
        | LC_ALL=C sort -f
}

CODEX_PROMPT_CHOICE_RESULT=""

codex_prompt_choice() {
    local prompt="${1:-choose> }" mode="${2:-freeform}" max_items="${3:-9}" reply rest old_tty status
    CODEX_PROMPT_CHOICE_RESULT=""
    codex_status_clear
    printf '%s' "$prompt" >&2
    if [ -t 0 ]; then
        old_tty="$(stty -g 2>/dev/null || true)"
        [ -z "$old_tty" ] || stty -echo -icanon min 1 time 0 2>/dev/null || true
        while :; do
            IFS= read -r -N 1 reply
            status=$?
            if [ "$status" -ne 0 ]; then
                [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                printf '\n' >&2
                return 1
            fi
            case "$reply" in
                $'\e')
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 130
                    ;;
                $'\n'|$'\r'|'')
                    [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                    printf '\n' >&2
                    return 0
                    ;;
                [0-9])
                    if [ "$mode" = "digits" ]; then
                        if [ "$reply" = "0" ] || [ "$reply" -le "$max_items" ]; then
                            [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                            printf '%s\n' "$reply" >&2
                            CODEX_PROMPT_CHOICE_RESULT="$reply"
                            return 0
                        fi
                        continue
                    fi
                    if [ "$max_items" -le 9 ]; then
                        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                        printf '%s\n' "$reply" >&2
                        CODEX_PROMPT_CHOICE_RESULT="$reply"
                        return 0
                    fi
                    ;;
                [yYnN])
                    if [ "$mode" = "yn" ]; then
                        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
                        printf '%s\n' "$reply" >&2
                        CODEX_PROMPT_CHOICE_RESULT="$reply"
                        return 0
                    fi
                    ;;
                *)
                    case "$mode" in
                        digits|yn) continue ;;
                    esac
                    break
                    ;;
            esac
            break
        done
        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
    elif ! IFS= read -r -N 1 reply; then
        printf '\n' >&2
        return 1
    fi
    case "$reply" in
        $'\e')
            printf '\n' >&2
            return 130
            ;;
        $'\n'|$'\r'|'')
            printf '\n' >&2
            return 0
            ;;
        [0-9])
            if [ "$mode" = "digits" ]; then
                if [ "$reply" = "0" ] || [ "$reply" -le "$max_items" ]; then
                    printf '%s\n' "$reply" >&2
                    CODEX_PROMPT_CHOICE_RESULT="$reply"
                    return 0
                fi
                return 1
            fi
            if [ "$max_items" -le 9 ]; then
                printf '%s\n' "$reply" >&2
                CODEX_PROMPT_CHOICE_RESULT="$reply"
                return 0
            fi
            ;;
        *)
        [ "$mode" = "digits" ] && return 1
            ;;
    esac
    if [ "$mode" = "yn" ]; then
        case "$reply" in
            [yYnN])
                printf '%s\n' "$reply" >&2
                CODEX_PROMPT_CHOICE_RESULT="$reply"
                return 0
                ;;
            '')
                return 0
                ;;
            *)
                return 1
                ;;
        esac
    fi
    rest=""
    printf '%s' "$reply" >&2
    IFS= read -r rest || true
    printf '%s%s\n' "$reply" "$rest" >&2
    CODEX_PROMPT_CHOICE_RESULT="$reply$rest"
    return 0
}

codex_prompt_interactive() {
    local prompt="$1" mode="${2:-freeform}" max_items="${3:-9}" empty_policy="${4:-keep}"
    local cancel_message="${5:-$(codex_ui_text_get selection_cancelled)}" status
    codex_prompt_choice "$(codex_ui_prompt "$prompt")" "$mode" "$max_items"
    status=$?
    case "$status" in
        130)
            [ -n "$cancel_message" ] && codex_say "$cancel_message"
            return 130
            ;;
        0)
            ;;
        *)
            return 1
            ;;
    esac
    if [ -z "${CODEX_PROMPT_CHOICE_RESULT:-}" ] && [ "$empty_policy" = "cancel" ]; then
        [ -n "$cancel_message" ] && codex_say "$cancel_message"
        return 130
    fi
    return 0
}

codex_confirm_menu() {
    local title="$1" subtitle="$2" yes_key="$3" yes_label="$4" yes_badges="$5"
    local no_key="$6" no_label="$7" no_badges="$8" prompt="$9" empty_policy="${10:-cancel}"
    codex_ui_menu_header "$title" "$subtitle"
    codex_ui_menu_row "$yes_key" "$yes_label" "$yes_badges"
    codex_ui_menu_row "$no_key" "$no_label" "$no_badges"
    printf '\n' >&2
    codex_prompt_interactive "$prompt" yn 0 "$empty_policy"
}

codex_profile_create_prompt() {
    local profile="$1" profile_dir="$2" display status
    display="$(codex_profile_display_name "$profile")"
    if [ ! -t 0 ] || [ ! -t 2 ]; then
        codex_fail "$(codex_ui_text_get missing_profile "$display")"
        return 2
    fi
    codex_prompt_interactive "$(codex_ui_text_get create_profile_prompt "$display")" yn 0 cancel "$(codex_ui_text_get profile_create_cancelled)" || {
        status=$?
        return "$status"
    }
    case "${CODEX_PROMPT_CHOICE_RESULT:-}" in
        y|Y)
            mkdir -p "$profile_dir"
            codex_say "$(codex_ui_text_get created_profile "$display")"
            return 0
            ;;
        *) return 130 ;;
    esac
}

codex_profile_ensure_dir() {
    local profile_dir="$1" profile="${2:-default}"
    if codex_profile_is_default "$profile"; then
        return 0
    fi
    if [ -d "$profile_dir" ]; then
        return 0
    fi
    codex_profile_create_prompt "$profile" "$profile_dir"
}

codex_profile_exec() {
    local profile_dir="$1" profile="${2:-default}"
    shift 2 || true
    codex_profile_ensure_dir "$profile_dir" "$profile" || return $?
    codex_ensure_runtime_ready || return $?
    codex_auto_update_if_needed || return $?
    codex_profile_runtime_exec "$profile" "$profile_dir" "$@"
}

codex_runtime_exec_with_context() {
    if [ -n "${CODEX_HOME:-}" ]; then
        codex_exec_current_runtime "$@"
        return $?
    fi
    local recent_profile recent_profile_dir
    recent_profile="$(codex_profile_read_recent)"
    recent_profile_dir="$(codex_profile_dir "$recent_profile")"
    codex_profile_runtime_exec "$recent_profile" "$recent_profile_dir" "$@"
}

codex_profile_list_command() {
    codex_status_clear
    printf 'default\n'
    codex_list_profiles
}

codex_profile_select() {
    local profiles=() profile choice idx profile_dir display_limit=0 truncated=0 recent
    recent="$(codex_profile_read_recent)"
    mapfile -t profiles < <(codex_profile_menu_ids)
    if [ -t 0 ]; then
        display_limit=9
    fi

    codex_ui_menu_header "$(codex_ui_text_get choose_profile_title)" "$(codex_ui_text_get choose_profile_subtitle)"
    idx=0
    for profile in "${profiles[@]}"; do
        if [ "$display_limit" -gt 0 ] && [ "$idx" -gt "$display_limit" ]; then
            truncated=1
            break
        fi
        if [ "$profile" = "$recent" ]; then
            printf '  %s %s %s\n' "$(codex_ui_number "$idx")" "$(codex_profile_display_name "$profile")" "$(codex_ui_badge recent)" >&2
        else
            printf '  %s %s\n' "$(codex_ui_number "$idx")" "$(codex_profile_display_name "$profile")" >&2
        fi
        idx=$((idx + 1))
    done
    if [ "$truncated" -eq 1 ]; then
        codex_ui_menu_note "$(codex_ui_text_get choose_profile_more)"
    fi
    printf '\n' >&2

    if [ ! -t 0 ]; then
        return 0
    fi

    codex_prompt_interactive "$(codex_ui_text_get choose_profile_prompt)" freeform "$(( ${#profiles[@]} < 9 ? ${#profiles[@]} : 9 ))" cancel || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"

    if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 0 ] && [ "$choice" -lt "${#profiles[@]}" ]; then
        profile="${profiles[$choice]}"
    else
        profile="$(codex_profile_choice_to_name "$choice")"
    fi

    codex_profile_validate_name "$profile" || {
        codex_fail "$(codex_ui_text_get invalid_profile "$profile")"
        return 2
    }
    profile_dir="$(codex_profile_dir "$profile")"
    codex_profile_exec "$profile_dir" "$profile"
}

codex_profile_run() {
    local profile="${1:-}"
    if [ -z "$profile" ]; then
        codex_profile_select
        return $?
    fi
    case "$profile" in
        list|ls)
            shift || true
            [ "$#" -eq 0 ] || {
                codex_fail "$(codex_ui_text_get profile_arg_error "$profile")"
                return 2
            }
            codex_profile_list_command
            return 0
            ;;
    esac
    codex_profile_validate_name "$profile" || {
        codex_fail "$(codex_ui_text_get invalid_profile "$profile")"
        return 2
    }
    local profile_dir
    profile_dir="$(codex_profile_dir "$profile")"
    shift || true
    codex_profile_exec "$profile_dir" "$profile" "$@"
}

codex_restore_backup() {
    local public="$1" base latest
    base="$(basename "$public")"
    latest="$(ls -t "$CODEX_TERMUX_BACKUP_DIR"/"$base".*.bak 2>/dev/null | sed -n '1p' || true)"
    if [ -n "$latest" ]; then
        cp -Pp "$latest" "$public"
        codex_say "$(codex_ui_text_get restored_backup "$public" "$latest")"
    fi
}

codex_remove() {
    if codex_file_has_marker "$CODEX_TERMUX_PUBLIC_CODEX"; then
        rm -f "$CODEX_TERMUX_PUBLIC_CODEX"
        codex_restore_backup "$CODEX_TERMUX_PUBLIC_CODEX"
    fi
    codex_rm_rf_managed "$CODEX_TERMUX_ROOT" || return $?
    codex_say "$(codex_ui_text_get removed_runtime "$CODEX_TERMUX_STATE_DIR")"
}

codex_use() {
    local choice="${1:-}" runtime_path version raw_sha runtime_sha package_spec
    if [ "$choice" = "--list" ]; then
        codex_use_list list
        return $?
    fi
    if [ -n "$choice" ]; then
        codex_use_select "$choice"
        return $?
    fi
    codex_use_list menu
    if [ ! -t 0 ]; then
        return 0
    fi
    codex_prompt_interactive "$(codex_ui_text_get choose_runtime_prompt)" digits "${CODEX_USE_MENU_COUNT:-0}" cancel || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    codex_use_select "$choice"
}

codex_use_list() {
    local mode="${1:-list}" latest interactive_limit=0
    latest="$(codex_latest_linux_arm64_version || true)"
    CODEX_USE_LAST_LATEST="$latest"
    if [ "$mode" = "menu" ] && [ -t 0 ]; then
        interactive_limit=9
    fi
    if [ "$mode" = "list" ]; then
        CODEX_USE_MENU_COUNT=0
        codex_use_render "$latest" "$interactive_limit" "$mode"
    else
        CODEX_USE_MENU_COUNT="$(codex_use_render "$latest" "$interactive_limit" "$mode")"
    fi
}

codex_use_render() {
    local latest="$1" interactive_limit="$2" mode="$3"
    codex_status_clear
    codex_termux_cmd use-render \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --latest "$latest" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --interactive-limit "$interactive_limit" \
        --mode "$mode"
}

codex_use_select() {
    local choice="$1" selected
    local latest="${CODEX_USE_LAST_LATEST:-}"
    if [ -z "$latest" ]; then
        latest="$(codex_latest_linux_arm64_version || true)"
    fi
    selected="$(codex_termux_cmd use-select \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --choice "$choice" \
        --latest "$latest" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --runtime-builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY")" || {
        codex_fail "Unknown runtime selection: $choice"
        return 1
    }
    local kind runtime_path raw_path version raw_sha runtime_sha package_spec
    IFS=$'\037' read -r kind runtime_path raw_path version raw_sha runtime_sha package_spec <<EOF
$selected
EOF
    if [ "$kind" = "remote" ]; then
        codex_runtime_install_upstream "$version" || return $?
    else
        codex_with_lock codex_activate_cached_runtime_unlocked \
            "$runtime_path" "$raw_path" "$version" "$raw_sha" "$runtime_sha" "$package_spec" || return $?
    fi
    codex_version || return $?
}

codex_session_share_source() {
    local source_path="$1" source_profile="${2:-default}" target_profile="${3:-default}"
    [ -n "$source_path" ] || return 0
    [ "$source_profile" = "$target_profile" ] && return 0
    [ -f "$source_path" ] || return 0

    local source_base target_base rel_path target_path
    if [ "$source_profile" = "default" ]; then
        source_base="$CODEX_TERMUX_HOME/.codex/sessions"
    else
        source_base="$CODEX_TERMUX_PROFILE_ROOT/$source_profile/sessions"
    fi
    if [ "$target_profile" = "default" ]; then
        target_base="$CODEX_TERMUX_HOME/.codex/sessions"
    else
        target_base="$CODEX_TERMUX_PROFILE_ROOT/$target_profile/sessions"
    fi

    case "$source_path" in
        "$source_base"/*) rel_path="${source_path#"$source_base"/}" ;;
        *) rel_path="$(basename "$source_path")" ;;
    esac
    target_path="$target_base/$rel_path"
    if [ -e "$target_path" ]; then
        return 0
    fi
    mkdir -p "$(dirname "$target_path")" || return $?
    ln -s "$source_path" "$target_path" 2>/dev/null || cp -p "$source_path" "$target_path"
}

codex_session() {
    local target_profile="" target_profile_dir=""
    if [ "$#" -gt 0 ] && [[ ! "$1" =~ ^- ]]; then
        target_profile="$1"
        shift
        codex_profile_validate_name "$target_profile" || {
            codex_fail "$(codex_ui_text_get invalid_profile "$target_profile")"
            return 2
        }
        target_profile_dir="$(codex_profile_dir "$target_profile")"
    fi

    local show_all="false"
    local arg
    for arg in "$@"; do
        if [ "$arg" = "--all" ]; then
            show_all="true"
            break
        fi
    done

    local tui_args=()
    if [ "$show_all" = "true" ]; then
        tui_args+=("--all")
    fi

    # Run Python session-tui with a temporary file for output to preserve TTY
    local temp_file
    temp_file="$(codex_mktemp_file codex-session)" || return $?
    
    CODEX_SESSION_TUI_DEFAULT_PROFILE="$target_profile" codex_termux_cmd session-tui --output "$temp_file" "${tui_args[@]}" || {
        local code=$?
        rm -f "$temp_file"
        if [ "$code" -eq 130 ]; then
            return 130
        fi
        return "$code"
    }

    if [ ! -s "$temp_file" ]; then
        rm -f "$temp_file"
        return 0
    fi

    local selected_plan
    selected_plan="$(cat "$temp_file")"
    rm -f "$temp_file"

    local native_session_ref source_profile workdir codex_home_env source_path
    IFS=$'\037' read -r target_profile target_profile_dir native_session_ref source_profile workdir codex_home_env source_path <<EOF
$selected_plan
EOF

    codex_session_share_source "$source_path" "$source_profile" "$target_profile"

    # Switch directory if workdir is specified and is a valid directory
    if [ -n "$workdir" ] && [ -d "$workdir" ]; then
        cd "$workdir" || true
    fi

    # Resume the session via wrapper's runtime execution path, forwarding any extra options
    codex_profile_exec "$target_profile_dir" "$target_profile" resume "$native_session_ref" "$@"
}

codex_install_source_command() {
    if [ -n "${CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-}" ] && [ -x "$CODEX_TERMUX_INSTALL_RUNTIME_SOURCE" ]; then
        printf '%s\n' "$CODEX_TERMUX_INSTALL_RUNTIME_SOURCE"
        return 0
    fi
    return 1
}

codex_run_install_source_command() {
    local source="$1" command="$2" tmp source_root snapshot status=0
    shift 2
    source_root="$(cd "$(dirname "$source")/.." && pwd)" || return 1
    tmp="$(codex_mktemp_dir codex-install-source)" || return 1
    snapshot="$tmp/source"
    cp -R "$source_root" "$snapshot" || {
        rm -rf "$tmp"
        return 1
    }
    bash "$snapshot/bin/install-runtime.sh" "$command" "$@" || status=$?
    rm -rf "$tmp"
    return "$status"
}

codex_install_public() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_run_install_source_command "$source" install "$@"
}

codex_update_full_public() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_run_install_source_command "$source" update "$@"
}

codex_repair_surface_public() {
    local source
    case "${1:-}" in
        "")
            ;;
        -h|--help|help)
            codex_wrapper_help
            return 0
            ;;
        *)
            codex_fail "repair does not take arguments"
            return 2
            ;;
    esac
    source="$(codex_install_source_command)" || {
        codex_repair_public "$@"
        return $?
    }
    codex_run_install_source_command "$source" repair "$@"
}

codex_notify_usage() {
    cat <<'USAGE'
Usage: codex notify [options]

Without options, opens an interactive notification setup prompt.

Options:
  --channel NAME          notification, toast, both
  --hooks LIST            Comma-separated hook list or "all"
  --hook NAME             Append a single hook name
  --all-hooks             Enable every supported hook position
  --pretooluse 0|1        Store legacy PreToolUse flag; use --hooks PreToolUse to enable the hook
  --content-chars N       Limit notification body to N characters
  --preserve-newlines 0|1 Keep notification body line breaks
  --toast-gravity VALUE   top, middle, or bottom
  --toast-short 0|1      Use short toast duration
  --toast-background HEX  Toast background color
  --toast-color HEX       Toast text color
  --group NAME            Notification group key
USAGE
}

codex_notify_interactive_usage() {
    cat <<'USAGE'
Usage: codex notify

Without options, opens an interactive notification setup prompt.
USAGE
}

codex_notify_need_arg() {
    [ $# -ge 2 ] || {
        codex_fail "Missing value for $1"
        return 64
    }
}

codex_notify_validate_bool() {
    local label="$1" value="$2"
    case "$value" in
        0|1) return 0 ;;
        *)
            codex_fail "$label must be 0 or 1"
            return 64
            ;;
    esac
}

codex_notify_validate_content_chars() {
    case "${1:-}" in
        0|full|none|unlimited) return 0 ;;
        [1-9]*)
            case "$1" in
                *[!0-9]*)
                    codex_fail "--content-chars must be a positive integer, 0, full, none, or unlimited"
                    return 64
                    ;;
                *)
                    return 0
                    ;;
            esac
            ;;
        *)
            codex_fail "--content-chars must be a positive integer, 0, full, none, or unlimited"
            return 64
            ;;
    esac
}

codex_notify_validate_hooks() {
    local hooks="${1:-Stop}" token event event_list=()
    case ",$hooks," in
        *,all,*|*,ALL,*)
            return 0
            ;;
    esac
    IFS=, read -r -a event_list <<<"$hooks"
    for token in "${event_list[@]}"; do
        event="$(codex_notify_hook_canonical "$token")"
        [ -n "$event" ] || continue
        if ! codex_notify_hook_valid "$event"; then
            codex_fail "Unknown notification hook: $token"
            return 64
        fi
    done
}

codex_notify_write_config() {
    local config_file="$1"
    shift
    mkdir -p "${config_file%/*}"
    {
        while [ $# -gt 1 ]; do
            printf '%s=%q\n' "$1" "$2"
            shift 2
        done
    } >"$config_file"
}

codex_notify_hook_ids() {
    codex_notify_all_hooks
}

codex_notify_render_hooks() {
    local idx=1 hook
    codex_ui_menu_header "Choose notify hooks" "Space-separated numbers or names, then Enter"
    while IFS= read -r hook; do
        printf '  %s %s\n' "$(codex_ui_number "$idx")" "$hook" >&2
        idx=$((idx + 1))
    done <<EOF
$(codex_notify_hook_ids)
EOF
    printf '  %s all\n' "$(codex_ui_number "0")" >&2
    printf '\n' >&2
}

codex_notify_parse_hook_selection() {
    local selection="${1:-}" token hook idx=0 hooks=() all_hooks=() found=0
    mapfile -t all_hooks < <(codex_notify_hook_ids)
    case "$selection" in
        "")
            printf 'Stop\n'
            return 0
            ;;
        all|ALL)
            printf 'all\n'
            return 0
            ;;
    esac
    for token in $selection; do
        case "$token" in
            0|all|ALL)
                printf 'all\n'
                return 0
                ;;
            *[!0-9]*)
                hook="$(codex_notify_hook_canonical "$token")"
                if ! codex_notify_hook_valid "$hook"; then
                    codex_fail "Unknown notification hook: $token"
                    return 64
                fi
                case "$hook" in
                    all)
                        printf 'all\n'
                        return 0
                        ;;
                esac
                case ",${hooks[*]:-}," in
                    *,"$hook",*) ;;
                    *) hooks+=("$hook") ;;
                esac
                found=1
                ;;
            [0-9]*)
                if [ "$token" -ge 1 ] && [ "$token" -le "${#all_hooks[@]}" ]; then
                    hook="${all_hooks[$((token - 1))]}"
                    case ",${hooks[*]:-}," in
                        *,"$hook",*) ;;
                        *) hooks+=("$hook") ;;
                    esac
                    found=1
                else
                    codex_fail "Notification hook number out of range: $token"
                    return 64
                fi
                ;;
            *)
                hook="$(codex_notify_hook_canonical "$token")"
                if ! codex_notify_hook_valid "$hook"; then
                    codex_fail "Unknown notification hook: $token"
                    return 64
                fi
                case "$hook" in
                    all)
                        printf 'all\n'
                        return 0
                        ;;
                esac
                case ",${hooks[*]:-}," in
                    *,"$hook",*) ;;
                    *) hooks+=("$hook") ;;
                esac
                found=1
                ;;
        esac
    done
    if [ "$found" -eq 0 ]; then
        printf 'Stop\n'
        return 0
    fi
    (IFS=,; printf '%s\n' "${hooks[*]}")
}

codex_notify_public() {
    local config_file="$CODEX_TERMUX_NOTIFY_CONFIG"
    local channel="${CODEX_TERMUX_NOTIFY_CHANNEL:-notification}"
    local hooks="${CODEX_TERMUX_NOTIFY_HOOKS:-Stop}"
    local pretooluse="${CODEX_TERMUX_NOTIFY_PRETOOLUSE:-0}"
    local content_chars="${CODEX_TERMUX_NOTIFY_CONTENT_CHARS:-140}"
    local preserve_newlines="${CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES:-0}"
    local toast_gravity="${CODEX_TERMUX_NOTIFY_TOAST_GRAVITY:-top}"
    local toast_short="${CODEX_TERMUX_NOTIFY_TOAST_SHORT:-0}"
    local toast_background="${CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND:-}"
    local toast_color="${CODEX_TERMUX_NOTIFY_TOAST_COLOR:-}"
    local group="${CODEX_TERMUX_NOTIFY_GROUP:-codex-turns}"
    if [ $# -eq 0 ]; then
        if [ -t 0 ] && [ -t 2 ]; then
            codex_notify_interactive_public
            return $?
        fi
        codex_fail "codex notify requires options or an interactive terminal"
        return 2
    fi
    while [ $# -gt 0 ]; do
        case "$1" in
            --help|-h)
                codex_notify_usage
                return 0
                ;;
            --config-file)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                config_file="${2:-}"
                shift 2
                ;;
            --channel)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                channel="${2:-}"
                shift 2
                ;;
            --hooks)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                hooks="${2:-}"
                shift 2
                ;;
            --hook)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                hooks="${hooks:+$hooks,}${2:-}"
                shift 2
                ;;
            --all-hooks)
                hooks="all"
                shift
                ;;
            --pretooluse)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                pretooluse="${2:-}"
                shift 2
                ;;
            --content-chars)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                content_chars="${2:-}"
                shift 2
                ;;
            --preserve-newlines)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                preserve_newlines="${2:-}"
                shift 2
                ;;
            --toast-gravity)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_gravity="${2:-}"
                shift 2
                ;;
            --toast-short)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_short="${2:-}"
                shift 2
                ;;
            --toast-background)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_background="${2:-}"
                shift 2
                ;;
            --toast-color)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                toast_color="${2:-}"
                shift 2
                ;;
            --group)
                codex_notify_need_arg "$1" "${2:-}" || return $?
                group="${2:-}"
                shift 2
                ;;
            *)
                codex_fail "Unknown notify option: $1"
                return 64
                ;;
        esac
    done
    codex_notify_validate_hooks "$hooks" || return $?
    codex_notify_validate_bool "--pretooluse" "$pretooluse" || return $?
    codex_notify_validate_content_chars "$content_chars" || return $?
    codex_notify_validate_bool "--preserve-newlines" "$preserve_newlines" || return $?
    codex_notify_validate_bool "--toast-short" "$toast_short" || return $?
    case "$toast_gravity" in
        ""|top|middle|bottom) ;;
        *)
            codex_fail "--toast-gravity must be top, middle, or bottom"
            return 64
            ;;
    esac
    [ -n "$config_file" ] || {
        codex_fail "Notification config file is unavailable"
        return 66
    }
    case "$channel" in
        toast|notification|both) ;;
        *)
            codex_fail "--channel must be notification, toast, or both"
            return 64
            ;;
    esac
    hooks="$(codex_notify_hooks_normalize "$hooks")"
    codex_notify_write_config "$config_file" \
        CODEX_TERMUX_NOTIFY_CONTENT_CHARS "$content_chars" \
        CODEX_TERMUX_NOTIFY_PRESERVE_NEWLINES "$preserve_newlines" \
        CODEX_TERMUX_NOTIFY_TOAST_GRAVITY "$toast_gravity" \
        CODEX_TERMUX_NOTIFY_TOAST_SHORT "$toast_short" \
        CODEX_TERMUX_NOTIFY_TOAST_BACKGROUND "$toast_background" \
        CODEX_TERMUX_NOTIFY_TOAST_COLOR "$toast_color" \
        CODEX_TERMUX_NOTIFY_GROUP "$group" \
        CODEX_TERMUX_NOTIFY_CHANNEL "$channel" \
        CODEX_TERMUX_NOTIFY_HOOKS "$hooks" \
        CODEX_TERMUX_NOTIFY_PRETOOLUSE "$pretooluse"
    codex_prepare_system_config || return $?
    codex_say "Saved notification settings to $config_file"
}

codex_notify_interactive_public() {
    local channel_choice hooks_choice gravity_choice channel hooks gravity
    codex_ui_menu_header "Configure notifications" "Choose channel, hooks, and toast position"
    printf '  %s notification\n' "$(codex_ui_number 1)" >&2
    printf '  %s toast\n' "$(codex_ui_number 2)" >&2
    printf '  %s both\n' "$(codex_ui_number 3)" >&2
    printf '\nChannel [3]> ' >&2
    IFS= read -r channel_choice || {
        codex_selection_cancelled
        return 130
    }
    case "${channel_choice:-3}" in
        1|notification) channel="notification" ;;
        2|toast) channel="toast" ;;
        3|both) channel="both" ;;
        *)
            codex_fail "Unknown notification channel selection: $channel_choice"
            return 64
            ;;
    esac

    codex_notify_render_hooks
    printf 'Hooks [Stop]> ' >&2
    IFS= read -r hooks_choice || {
        codex_selection_cancelled
        return 130
    }
    hooks="$(codex_notify_parse_hook_selection "$hooks_choice")" || return $?

    gravity="top"
    case "$channel" in
        toast|both)
            printf 'Toast gravity [top]> ' >&2
            IFS= read -r gravity_choice || {
                codex_selection_cancelled
                return 130
            }
            gravity="${gravity_choice:-top}"
            ;;
    esac

    codex_notify_public --channel "$channel" --hooks "$hooks" --toast-gravity "$gravity"
}

codex_setup_public() {
    codex_status_clear
    printf 'Error: %s\n' "$(codex_ui_text_get setup_reserved)" >&2
    return 2
}

codex_main() {
    local recent_profile recent_profile_dir status=0
    case "${1:-}" in
        setup)
            shift
            codex_setup_public "$@"
            ;;
        install)
            shift
            codex_install_public "$@"
            ;;
        notify)
            shift
            codex_notify_public "$@"
            ;;
        update)
            shift
            codex_update_full_public "$@"
            ;;
        repair)
            shift
            codex_repair_surface_public "$@"
            ;;
        doctor)
            shift
            codex_public_doctor "$@"
            ;;
        version)
            shift
            case "${1:-}" in
                "")
                    codex_version
                    ;;
                -h|--help|help)
                    codex_wrapper_help
                    ;;
                *)
                    codex_fail "version does not take arguments"
                    return 2
                    ;;
            esac
            ;;
        help|--help|-h)
            shift || true
            codex_help "$@"
            ;;
        use)
            shift
            codex_use "$@"
            ;;
        session)
            shift
            codex_session "$@"
            ;;
        profile)
            shift
            codex_profile_run "$@"
            ;;
        remove)
            shift
            codex_remove
            ;;
        *)
            if ! codex_ensure_runtime_ready; then
                status=$?
            elif ! codex_auto_update_if_needed; then
                status=$?
            else
                codex_runtime_exec_with_context "$@"
            fi
            ;;
    esac
    status=$?
    codex_status_clear
    return "$status"
}
