#!/usr/bin/env bash
set -u

CODEX_TERMUX_SHELL_DIR="${BASH_SOURCE[0]%/*}"
[ "$CODEX_TERMUX_SHELL_DIR" = "${BASH_SOURCE[0]}" ] && CODEX_TERMUX_SHELL_DIR="."
CODEX_TERMUX_SHELL_DIR="$(cd "$CODEX_TERMUX_SHELL_DIR" && pwd)"

CODEX_TERMUX_HOME="${CODEX_TERMUX_HOME:-$HOME}"
CODEX_TERMUX_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_TERMUX_ROOT="${CODEX_TERMUX_ROOT:-$CODEX_TERMUX_HOME/.local/lib/codex/termux}"
CODEX_TERMUX_MANAGER_DIR="${CODEX_TERMUX_MANAGER_DIR:-$CODEX_TERMUX_ROOT/manager}"
CODEX_TERMUX_RAW_DIR="${CODEX_TERMUX_RAW_DIR:-$CODEX_TERMUX_ROOT/raw}"
CODEX_TERMUX_RAW_VENDOR="${CODEX_TERMUX_RAW_VENDOR:-$CODEX_TERMUX_RAW_DIR/vendor/aarch64-unknown-linux-musl}"
CODEX_TERMUX_RUNTIME_DIR="${CODEX_TERMUX_RUNTIME_DIR:-$CODEX_TERMUX_ROOT/current}"
CODEX_TERMUX_CURRENT_LINK="${CODEX_TERMUX_CURRENT_LINK:-$CODEX_TERMUX_RUNTIME_DIR}"
CODEX_TERMUX_VERIFIED_LINK="${CODEX_TERMUX_VERIFIED_LINK:-$CODEX_TERMUX_ROOT/verified}"
CODEX_TERMUX_RUNTIME="${CODEX_TERMUX_RUNTIME:-$CODEX_TERMUX_RUNTIME_DIR/codex}"
CODEX_TERMUX_MANAGED_SHELL="${CODEX_TERMUX_MANAGED_SHELL:-$CODEX_TERMUX_MANAGER_DIR/managed.sh}"
CODEX_TERMUX_STATE_DIR="${CODEX_TERMUX_STATE_DIR:-$CODEX_TERMUX_HOME/.local/share/codex/termux}"
CODEX_TERMUX_PROFILE_ROOT="${CODEX_TERMUX_PROFILE_ROOT:-$CODEX_TERMUX_HOME/.codex-profiles}"
CODEX_TERMUX_SHARED_PLUGINS_DIR="${CODEX_TERMUX_SHARED_PLUGINS_DIR:-$CODEX_TERMUX_HOME/.codex/plugins}"
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
CODEX_TERMUX_CERT_FILE="${CODEX_TERMUX_CERT_FILE:-$CODEX_TERMUX_PREFIX/etc/tls/cert.pem}"
CODEX_TERMUX_CERT_DIR="${CODEX_TERMUX_CERT_DIR:-$CODEX_TERMUX_PREFIX/etc/tls/certs}"
CODEX_TERMUX_RESOLVER_FD="${CODEX_TERMUX_RESOLVER_FD:-33}"
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
CODEX_TERMUX_LAST_PROFILE_FILE="${CODEX_TERMUX_LAST_PROFILE_FILE:-$CODEX_TERMUX_STATE_DIR/last-profile}"
CODEX_TERMUX_RUNTIME_RETENTION="${CODEX_TERMUX_RUNTIME_RETENTION:-3}"
CODEX_TERMUX_PATCH_POLICY="${CODEX_TERMUX_PATCH_POLICY:-dns-fd33-only-v1}"

codex_say() { printf 'codex: %s\n' "$*" >&2; }
codex_fail() { printf 'codex: ERROR: %s\n' "$*" >&2; return 1; }

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

codex_ui_bold() {
    printf '%s' "$1"
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
    printf '%s\n' "$title" >&2
    if [ -n "$subtitle" ]; then
        printf '%s\n' "$(codex_ui_dim "$subtitle")" >&2
    fi
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
        codex_fail "internal helper package is unavailable"
        return 1
    fi
    printf '%s\n' "$package_root"
}

codex_termux_cmd() {
    local package_root
    package_root="$(codex_termux_package_root)" || return 1
    PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" python3 -m codex_termux.cli "$@"
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

codex_with_lock() {
    local cmd="$1"
    shift
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    if command -v flock >/dev/null 2>&1; then
        (
            if ! flock -w "$CODEX_TERMUX_LOCK_WAIT_SECONDS" -x 9; then
                codex_fail "another mutation operation is in progress: $CODEX_TERMUX_LOCK_FILE"
                exit 75
            fi
            "$cmd" "$@"
        ) 9>"$CODEX_TERMUX_LOCK_FILE"
    else
        local lock_dir="${CODEX_TERMUX_LOCK_FILE}.d" waited=0
        while ! mkdir "$lock_dir" 2>/dev/null; do
            sleep 1
            waited=$((waited + 1))
            if [ "$waited" -ge "$CODEX_TERMUX_LOCK_WAIT_SECONDS" ]; then
                codex_fail "another mutation operation is in progress: $CODEX_TERMUX_LOCK_FILE"
                return 75
            fi
        done
        (
            trap 'rmdir "$lock_dir" 2>/dev/null || true' EXIT
            "$cmd" "$@"
        )
    fi
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
        XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$run_home/.config}" \
        XDG_CACHE_HOME="${XDG_CACHE_HOME:-$run_home/.cache}" \
        XDG_DATA_HOME="${XDG_DATA_HOME:-$run_home/.local/share}" \
        GODEBUG="${GODEBUG:-netdns=go}" \
        SSL_CERT_FILE="$CODEX_TERMUX_CERT_FILE" \
        CODEX_SELF_EXE="$executable" \
        CODEX_TERMUX_BWRAP_COMPAT_QUIET="${CODEX_TERMUX_BWRAP_COMPAT_QUIET:-1}" \
        PATH="$runtime_dir/codex-path:$runtime_dir/codex-resources:$CODEX_TERMUX_PREFIX/bin:$PATH" \
        "${cert_dir_env[@]}")
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
    fi
    "${runtime_env[@]}" "$executable" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF"
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
    rm -rf "$backup"
    if [ -e "$target" ] || [ -L "$target" ]; then
        mv "$target" "$backup" || return 1
        existed=1
    fi
    if mv "$source" "$target"; then
        rm -rf "$backup"
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
    codex_validate_runtime_retention || return $?
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
    rm -rf "$complete_dir"
    mkdir -p "$complete_dir"
    for name in codex codex-resources codex-path codex-package.json runtime-build.json; do
        [ -e "$payload_dir/$name" ] || {
            rm -rf "$complete_dir"
            return 1
        }
        cp -R "$payload_dir/$name" "$complete_dir/$name"
    done
    [ -x "$CODEX_TERMUX_RUNTIME_BUILDER" ] &&
        [ -r "$support_dir/bwrap-termux-compat.py" ] &&
        [ -r "$support_dir/rg-termux-shim.sh" ] || {
        rm -rf "$complete_dir"
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
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/codex-pack.XXXXXX")" || return 1
    pack_json="$tmp/pack.json"
    codex_say "fetching $package_spec"
    codex_say "then unpack, build, smoke-test, and publish"
    if ! npm pack "$package_spec" --json --pack-destination "$tmp" >"$pack_json"; then
        rm -rf "$tmp"
        codex_fail "npm pack failed for $package_spec"
        return 1
    fi
    filename="$(codex_extract_pack_field "$pack_json" filename)"
    version="$(codex_extract_pack_field "$pack_json" version)"
    tgz="$tmp/$filename"
    if [ ! -f "$tgz" ]; then
        rm -rf "$tmp"
        codex_fail "npm pack did not create expected tarball"
        return 1
    fi
    codex_say "validating package archive"
    mkdir -p "$tmp/package"
    if ! codex_validate_tarball_safe "$tgz" >/dev/null 2>&1; then
        rm -rf "$tmp"
        codex_fail "unsafe tarball contents in $tgz"
        return 1
    fi
    codex_say "unpacking package archive"
    if ! tar -xzf "$tgz" -C "$tmp/package" --strip-components=1; then
        rm -rf "$tmp"
        codex_fail "failed to extract $tgz"
        return 1
    fi
    printf '%s\t%s\t%s\t%s\n' "$tmp" "$tmp/package/vendor/aarch64-unknown-linux-musl" "$version" "$package_spec"
}

codex_install_raw_vendor() {
    local src_vendor="$1" target_dir="${2:-$CODEX_TERMUX_RAW_DIR}" staged
    staged="$target_dir.new.$$"
    rm -rf "$staged"
    mkdir -p "$staged/vendor"
    cp -R "$src_vendor" "$staged/vendor/aarch64-unknown-linux-musl"
    chmod 755 "$staged/vendor/aarch64-unknown-linux-musl/bin/codex"
    if ! codex_replace_tree_atomic "$staged" "$target_dir" "$target_dir.old"; then
        rm -rf "$staged"
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

codex_rebuild_runtime_unlocked() {
    local version="${1:-unknown}" package_spec="${2:-local}" report build_stdout raw_sha runtime_sha
    local runtime_stage="$CODEX_TERMUX_RUNTIME_DIR.build.$$" runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.complete.$$"
    [ -x "$CODEX_TERMUX_RAW_VENDOR/bin/codex" ] || return 1
    mkdir -p "$CODEX_TERMUX_STATE_DIR" "$CODEX_TERMUX_DOCTOR_DIR"
    report="$CODEX_TERMUX_DOCTOR_DIR/last-build-report.json"
    build_stdout="$CODEX_TERMUX_DOCTOR_DIR/last-build-report.stdout"
    rm -rf "$runtime_stage"
    if [ "${CODEX_TERMUX_BUILD_VERBOSE:-0}" = "1" ]; then
        if ! "$CODEX_TERMUX_RUNTIME_BUILDER" "$CODEX_TERMUX_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report"; then
            rm -rf "$runtime_stage"
            return 1
        fi
    else
        if ! "$CODEX_TERMUX_RUNTIME_BUILDER" "$CODEX_TERMUX_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report" >"$build_stdout" 2>&1; then
            rm -rf "$runtime_stage"
            return 1
        fi
    fi
    codex_say "preparing runtime bundle"
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        rm -rf "$runtime_stage" "$runtime_complete"
        return 1
    fi
    raw_sha="$(codex_sha256 "$CODEX_TERMUX_RAW_VENDOR/bin/codex")"
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    rm -rf "$runtime_stage"
    codex_say "smoke-testing runtime"
    if ! codex_smoke_test_runtime "$runtime_complete/codex"; then
        rm -rf "$runtime_complete"
        return 1
    fi
    codex_say "publishing runtime and refreshing pointers"
    codex_commit_runtime_candidate "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec"
}

codex_repair_runtime_from_raw_unlocked() {
    local version package_spec
    codex_raw_integrity_ok || {
        codex_fail "cached raw package integrity check failed; run codex update"
        return 1
    }
    version="$(codex_read_state_field version)"
    package_spec="$(codex_read_state_field package_spec)"
    [ -n "$version" ] || version="unknown"
    [ -n "$package_spec" ] || package_spec="local"
    codex_rebuild_runtime_unlocked "$version" "$package_spec" || return $?
    codex_say "runtime repaired from cached raw package ($version)"
}

codex_repair_runtime_from_raw() {
    codex_with_lock codex_repair_runtime_from_raw_unlocked
}

codex_update_unlocked() {
    local requested="${1:-}" fetched tmp vendor version spec raw_stage runtime_stage runtime_complete raw_sha runtime_sha
    fetched="$(codex_fetch_package "$requested")" || return $?
    IFS=$'\t' read -r tmp vendor version spec <<EOF
$fetched
EOF
    raw_stage="$CODEX_TERMUX_RAW_DIR.update.$$"
    runtime_stage="$CODEX_TERMUX_RUNTIME_DIR.update.$$"
    runtime_complete="$CODEX_TERMUX_RUNTIME_DIR.complete.$$"
    mkdir -p "$CODEX_TERMUX_DOCTOR_DIR"
    codex_say "staging raw vendor tree"
    if ! codex_install_raw_vendor "$vendor" "$raw_stage"; then
        rm -rf "$tmp"
        return 1
    fi
    codex_say "building patched runtime"
    if ! codex_build_runtime_tree "$raw_stage/vendor/aarch64-unknown-linux-musl" "$runtime_stage" "$CODEX_TERMUX_DOCTOR_DIR/last-build-report.stdout"; then
        rm -rf "$tmp" "$raw_stage" "$runtime_stage" "$runtime_complete"
        return 1
    fi
    codex_say "preparing runtime bundle"
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        rm -rf "$tmp" "$raw_stage" "$runtime_stage" "$runtime_complete"
        return 1
    fi
    raw_sha="$(codex_sha256 "$raw_stage/vendor/aarch64-unknown-linux-musl/bin/codex")"
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    rm -rf "$runtime_stage"
    codex_say "smoke-testing runtime"
    if ! codex_smoke_test_runtime "$runtime_complete/codex"; then
        rm -rf "$tmp" "$raw_stage" "$runtime_complete"
        return 1
    fi
    codex_say "publishing runtime and refreshing pointers"
    if ! codex_commit_runtime_candidate "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$spec" "$raw_stage"; then
        rm -rf "$tmp" "$raw_stage" "$runtime_complete"
        return 1
    fi
    rm -rf "$tmp"
    codex_say "installed Codex $version"
}

codex_update() {
    codex_validate_runtime_retention || return $?
    codex_with_lock codex_update_unlocked "${1:-}"
}

codex_update_public() {
    local installed_version
    codex_update "${1:-}" || return $?
    installed_version="$(codex_read_state_field version)"
    codex_version || return $?
    codex_prompt_update_launch "$installed_version"
}

codex_update_launch_selected() {
    local installed_version="${1:-unknown}" recent_profile recent_profile_dir
    case "${CODEX_PROMPT_CHOICE_RESULT:-}" in
        y|Y)
            recent_profile="$(codex_profile_read_recent)"
            recent_profile_dir="$(codex_profile_dir "$recent_profile")"
            codex_say "launching Codex $(codex_display_version "$installed_version")"
            codex_profile_runtime_exec "$recent_profile" "$recent_profile_dir"
            ;;
    esac
    return 0
}

codex_prompt_update_launch() {
    local installed_version="$1" display_installed
    [ -t 0 ] && [ -t 2 ] || return 0
    display_installed="$(codex_display_version "$installed_version")"
    codex_ui_menu_header "Codex update complete" "$display_installed is installed"
    codex_ui_menu_row y "run now" "$(codex_ui_badge run)"
    codex_ui_menu_row N "stay here" "$(codex_ui_badge keep)"
    printf '\n' >&2
    codex_prompt_choice "$(codex_ui_prompt 'run Codex now [y/N]> ')" yn 0
    case "$?" in
        0)
            codex_update_launch_selected "$display_installed"
            ;;
        *)
            return 0
            ;;
    esac
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
    codex_ui_menu_header "Codex update available" "$display_current -> $display_latest"
    codex_ui_menu_row y "$display_latest" "$(codex_ui_badge update)"
    codex_ui_menu_row N "$display_current" "$(codex_ui_badge current)" "$(codex_ui_badge keep)"
    printf '\n' >&2
    codex_prompt_choice "$(codex_ui_prompt 'codex update [y/N]> ')" yn 0
    case "$?" in
        130)
            return 130
            ;;
        0)
            ;;
        *)
            return 1
            ;;
    esac
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    case "$choice" in
        y|Y)
            return 0
            ;;
        *)
            codex_say "keeping current patched runtime ($(codex_display_version "$current"))"
            return 1
            ;;
    esac
}

codex_install_auto_update() {
    local current="$1" latest="$2"
    codex_say "updating: $current -> $latest"
    if codex_update "$latest"; then
        codex_clear_pending_auto_update
        codex_clear_failed_auto_update
    else
        codex_write_failed_auto_update "$latest"
        codex_say "update failed; continuing with $current"
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
        rm -rf "$runtime_complete" "$raw_complete"
        return 1
    fi
    if ! codex_commit_runtime_candidate "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$raw_complete"; then
        rm -rf "$runtime_complete" "$raw_complete"
        return 1
    fi
    codex_say "using Codex $version"
}


codex_try_verified_rollback_unlocked() {
    codex_termux_activation_cmd activation-restore-verified >/dev/null || return 1
    codex_say "active runtime restored from verified tuple"
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
            codex_fail "cached raw package integrity check failed; run codex update"
            return 1
        fi
        codex_say "runtime drift detected; rebuilding from cached raw package"
        codex_repair_runtime_from_raw
        return $?
    fi
    codex_fail "runtime missing and no cached raw package is available; run codex setup"
    return 127
}


codex_prepare_runtime_env() {
    export SSL_CERT_FILE="${SSL_CERT_FILE:-$CODEX_TERMUX_CERT_FILE}"
    [ -d "$CODEX_TERMUX_CERT_DIR" ] && export SSL_CERT_DIR="${SSL_CERT_DIR:-$CODEX_TERMUX_CERT_DIR}"
    if [ -z "${BROWSER:-}" ] && command -v termux-open-url >/dev/null 2>&1; then
        export BROWSER=termux-open-url
    fi
    export CODEX_SELF_EXE="${CODEX_SELF_EXE:-$CODEX_TERMUX_RUNTIME}"
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    export CODEX_TERMUX_BWRAP_COMPAT_QUIET="${CODEX_TERMUX_BWRAP_COMPAT_QUIET:-1}"
    export PATH="$CODEX_TERMUX_RUNTIME_DIR/codex-path:$CODEX_TERMUX_RUNTIME_DIR/codex-resources:$CODEX_TERMUX_PREFIX/bin:$PATH"
    if [ -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        eval "exec ${CODEX_TERMUX_RESOLVER_FD}<\"\$CODEX_TERMUX_RESOLV_CONF\""
    fi
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

codex_version() {
    local upstream wrapper commit status=0
    if upstream="$("$CODEX_TERMUX_RUNTIME" --version 2>/dev/null)"; then
        status=0
    else
        status=$?
        upstream=""
    fi
    wrapper="$(codex_current_wrapper_version)"
    commit="$(codex_current_wrapper_commit)"
    printf 'codex  : %s\n' "${upstream:-missing}"
    printf 'wrapper: %s (%s)\n' "$wrapper" "$commit"
    return "$status"
}

codex_wrapper_help() {
    printf '\n'
    printf 'Wrapper commands\n'
    printf '  %-8s  %s\n' 'codex' 'Managed upstream Codex entrypoint; bare execution may auto-update before launch.'
    printf '  %-8s  %s\n' 'setup' 'Refresh launcher/support files and ensure raw/runtime are ready.'
    printf '  %-8s  %s\n' 'update' 'Refresh support, update official linux-arm64 package, patch, and promote.'
    printf '  %-8s  %s\n' 'use' 'List cached and remote runtimes; promote the selected runtime.'
    printf '  %-8s  %s\n' 'profile' 'List numbered profiles or enter a named profile with CODEX_HOME switched.'
    printf '  %-8s  %s\n' 'doctor' 'Check launcher, runtime resources, resolver, CA, DNS patch, and state.'
    printf '  %-8s  %s\n' 'version' 'Print `codex :` and `wrapper:` version rows.'
    printf '  %-8s  %s\n' 'remove' 'Remove managed launcher/runtime and restore launcher backups when present.'
}

codex_help() {
    if [ -x "$CODEX_TERMUX_RUNTIME" ]; then
        codex_prepare_runtime_env
        "$CODEX_TERMUX_RUNTIME" --help
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
    if [ "${1:-}" = "--json" ]; then
        codex_wrapper_doctor_json
    else
        codex_wrapper_doctor_json | codex_termux_cmd doctor-render --mode human
    fi
}

codex_public_doctor() {
    if [ $# -gt 0 ]; then
        codex_ensure_runtime_ready || return $?
        codex_prepare_runtime_env
        "$CODEX_TERMUX_RUNTIME" doctor "$@"
        return $?
    fi
    local upstream_status=0 wrapper_status=0
    codex_ensure_runtime_ready || return $?
    codex_prepare_runtime_env
    "$CODEX_TERMUX_RUNTIME" doctor || upstream_status=$?
    printf '\n%s\n\n' '─────────────────────────────────────────────────────────────'
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
    codex_say "entering profile $(codex_profile_display_name "$profile")"
}

codex_profile_runtime_exec() {
    local profile="$1" profile_dir="$2"
    shift 2 || true
    codex_profile_write_recent "$profile"
    codex_profile_note "$profile"
    codex_prepare_runtime_env
    if codex_profile_is_default "$profile"; then
        exec "$CODEX_TERMUX_RUNTIME" "$@"
    fi
    codex_profile_share_plugins "$profile_dir"
    CODEX_HOME="$profile_dir" exec "$CODEX_TERMUX_RUNTIME" "$@"
}

codex_profile_menu_ids() {
    local recent profile
    recent="$(codex_profile_read_recent)"
    printf 'default\n'
    if [ "$recent" != "default" ]; then
        printf '%s\n' "$recent"
    fi
    while IFS= read -r profile; do
        [ "$profile" = "default" ] && continue
        [ "$profile" = "$recent" ] && continue
        printf '%s\n' "$profile"
    done < <(codex_list_profiles)
}

codex_profile_share_plugins() {
    local profile_dir="$1" shared_plugins_dir="$CODEX_TERMUX_SHARED_PLUGINS_DIR" plugins_dir
    plugins_dir="$profile_dir/plugins"
    mkdir -p "$profile_dir" "$shared_plugins_dir"
    if [ -e "$plugins_dir" ] || [ -L "$plugins_dir" ]; then
        return 0
    fi
    ln -s "$shared_plugins_dir" "$plugins_dir"
}

codex_list_profiles() {
    local root="$CODEX_TERMUX_PROFILE_ROOT"
    [ -d "$root" ] || return 0
    find "$root" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null \
        | grep -Ev '^(default|termux)$' \
        | grep -Ev '^[.]' \
        | LC_ALL=C sort -f
}

CODEX_PROMPT_CHOICE_RESULT=""

codex_prompt_choice() {
    local prompt="${1:-choose> }" mode="${2:-freeform}" max_items="${3:-9}" reply rest old_tty status
    CODEX_PROMPT_CHOICE_RESULT=""
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

codex_profile_create_prompt() {
    local profile="$1" profile_dir="$2" display status
    display="$(codex_profile_display_name "$profile")"
    if [ ! -t 0 ] || [ ! -t 2 ]; then
        codex_fail "profile does not exist: $display"
        return 2
    fi
    codex_prompt_choice "$(codex_ui_prompt "profile '$display' does not exist. Create it? [y/N] ")" yn 0
    status=$?
    if [ "$status" -ne 0 ]; then
        codex_say "profile creation cancelled"
        return "$status"
    fi
    case "${CODEX_PROMPT_CHOICE_RESULT:-}" in
        y|Y)
            mkdir -p "$profile_dir"
            codex_say "created profile $display"
            return 0
            ;;
        *)
            codex_say "profile creation cancelled"
            return 130
            ;;
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

codex_profile_select() {
    local profiles=() profile choice idx profile_dir display_limit=0 truncated=0 recent
    recent="$(codex_profile_read_recent)"
    mapfile -t profiles < <(codex_profile_menu_ids)
    if [ -t 0 ]; then
        display_limit=9
    fi

    printf '%s\n' "Choose profile" >&2
    printf '%s\n' "$(codex_ui_dim 'Select CODEX_HOME target')" >&2
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
        printf '%s\n' "$(codex_ui_dim '  (More options: codex profile NAME)')" >&2
    fi
    printf '\n' >&2

    if [ ! -t 0 ]; then
        return 0
    fi

    codex_prompt_choice "$(codex_ui_prompt 'choose profile > ')" freeform "$(( ${#profiles[@]} < 9 ? ${#profiles[@]} : 9 ))" || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    [ -n "$choice" ] || return 130

    if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 0 ] && [ "$choice" -lt "${#profiles[@]}" ]; then
        profile="${profiles[$choice]}"
    else
        profile="$(codex_profile_choice_to_name "$choice")"
    fi

    codex_profile_validate_name "$profile" || {
        codex_fail "invalid profile name: $profile"
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
    codex_profile_validate_name "$profile" || {
        codex_fail "invalid profile name: $profile"
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
        codex_say "restored $public from $latest"
    fi
}

codex_remove() {
    if codex_file_has_marker "$CODEX_TERMUX_PUBLIC_CODEX"; then
        rm -f "$CODEX_TERMUX_PUBLIC_CODEX"
        codex_restore_backup "$CODEX_TERMUX_PUBLIC_CODEX"
    fi
    rm -rf "$CODEX_TERMUX_ROOT"
    codex_say "removed managed runtime; state kept at $CODEX_TERMUX_STATE_DIR for backups"
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
    codex_prompt_choice "$(codex_ui_prompt 'choose runtime > ')" digits "${CODEX_USE_MENU_COUNT:-0}" || return $?
    choice="$CODEX_PROMPT_CHOICE_RESULT"
    if [ -z "$choice" ]; then
        printf 'codex use: cancelled.\n' >&2
        return 1
    fi
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
        codex_fail "unknown runtime selection: $choice"
        return 1
    }
    local kind runtime_path raw_path version raw_sha runtime_sha package_spec
    IFS=$'\037' read -r kind runtime_path raw_path version raw_sha runtime_sha package_spec <<EOF
$selected
EOF
    if [ "$kind" = "remote" ]; then
        codex_update "$version" || return $?
    else
        codex_with_lock codex_activate_cached_runtime_unlocked \
            "$runtime_path" "$raw_path" "$version" "$raw_sha" "$runtime_sha" "$package_spec" || return $?
    fi
    codex_version || return $?
}

codex_setup_public() {
    if [ -n "${CODEX_TERMUX_INSTALL_RUNTIME_SOURCE:-}" ] && [ -x "$CODEX_TERMUX_INSTALL_RUNTIME_SOURCE" ]; then
        exec bash "$CODEX_TERMUX_INSTALL_RUNTIME_SOURCE" setup "$@"
    fi
    codex_update "${1:-}"
}

codex_main() {
    local recent_profile recent_profile_dir
    case "${1:-}" in
        setup)
            shift
            codex_setup_public "$@"
            ;;
        update)
            shift
            codex_update_public "${1:-}"
            ;;
        doctor)
            shift
            codex_public_doctor "$@"
            ;;
        version)
            shift
            codex_version
            ;;
        help|--help|-h)
            shift || true
            codex_help "$@"
            ;;
        use)
            shift
            codex_use "$@"
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
            codex_ensure_runtime_ready || return $?
            codex_auto_update_if_needed || return $?
            recent_profile="$(codex_profile_read_recent)"
            recent_profile_dir="$(codex_profile_dir "$recent_profile")"
            codex_profile_runtime_exec "$recent_profile" "$recent_profile_dir" "$@"
            ;;
    esac
}
