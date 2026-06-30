# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

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

codex_load_wrapper_source_config() {
    [ -r "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" ] || return 0
    # shellcheck disable=SC1090
    . "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG"
    codex_normalize_wrapper_source_config
}

codex_normalize_wrapper_source_config() {
    if [ -z "${CODEX_TERMUX_WRAPPER_REPO:-}" ] && [ -n "${CODEX_TERMUX_WRAPPER_GIT_REPO:-}" ]; then
        CODEX_TERMUX_WRAPPER_REPO="$CODEX_TERMUX_WRAPPER_GIT_REPO"
        export CODEX_TERMUX_WRAPPER_REPO
    fi
    if [ -z "${CODEX_TERMUX_WRAPPER_REF:-}" ] && [ -n "${CODEX_TERMUX_WRAPPER_GIT_REF:-}" ]; then
        CODEX_TERMUX_WRAPPER_REF="$CODEX_TERMUX_WRAPPER_GIT_REF"
        export CODEX_TERMUX_WRAPPER_REF
    fi
    if [ -z "${CODEX_TERMUX_WRAPPER_TOKEN:-}" ]; then
        if [ -n "${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}" ]; then
            CODEX_TERMUX_WRAPPER_TOKEN="$CODEX_TERMUX_WRAPPER_GIT_TOKEN"
        elif [ -n "${CODEX_TERMUX_WRAPPER_RELEASE_TOKEN:-}" ]; then
            CODEX_TERMUX_WRAPPER_TOKEN="$CODEX_TERMUX_WRAPPER_RELEASE_TOKEN"
        fi
        [ -z "${CODEX_TERMUX_WRAPPER_TOKEN:-}" ] || export CODEX_TERMUX_WRAPPER_TOKEN
    fi
}

codex_wrapper_auth_token() {
    local token
    codex_normalize_wrapper_source_config
    token="${CODEX_TERMUX_WRAPPER_TOKEN:-}"
    [ -n "$token" ] || token="${GITHUB_TOKEN:-}"
    if [ -z "$token" ] && command -v gh >/dev/null 2>&1; then
        token="$(gh auth token 2>/dev/null || true)"
    fi
    [ -n "$token" ] || return 1
    printf '%s\n' "$token"
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
        choose_profile_more) printf '  (More options: codex termux profile NAME)\n' ;;
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
        setup_reserved) printf 'The upstream setup command is reserved. Use codex termux install, update, repair, or notify for wrapper operations.\n' ;;
        doctor_wrapper_title) printf 'Wrapper doctor\n' ;;
        session_stub) printf 'Use codex termux session for the cross-profile session picker.\n' ;;
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
    source_root="$CODEX_TERMUX_WRAPPER_ROOT"
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
    local action="$1" shell_lib="$CODEX_TERMUX_SHELL_LIB" wrapper_version wrapper_commit
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
