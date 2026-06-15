#!/usr/bin/env bash
set -u

CODEX_NATIVE_SHELL_DIR="${BASH_SOURCE[0]%/*}"
[ "$CODEX_NATIVE_SHELL_DIR" = "${BASH_SOURCE[0]}" ] && CODEX_NATIVE_SHELL_DIR="."
CODEX_NATIVE_SHELL_DIR="$(cd "$CODEX_NATIVE_SHELL_DIR" && pwd)"

CODEX_NATIVE_HOME="${CODEX_NATIVE_HOME:-$HOME}"
CODEX_NATIVE_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_NATIVE_NATIVE_ROOT="${CODEX_NATIVE_NATIVE_ROOT:-$CODEX_NATIVE_HOME/.local/lib/codex/native}"
CODEX_NATIVE_MANAGER_DIR="${CODEX_NATIVE_MANAGER_DIR:-$CODEX_NATIVE_NATIVE_ROOT/manager}"
CODEX_NATIVE_LEGACY_RUNTIME_DIR="${CODEX_NATIVE_LEGACY_RUNTIME_DIR:-$CODEX_NATIVE_NATIVE_ROOT/runtime}"
CODEX_NATIVE_LEGACY_RAW_DIR="${CODEX_NATIVE_LEGACY_RAW_DIR:-$CODEX_NATIVE_NATIVE_ROOT/raw}"
CODEX_NATIVE_RAW_DIR="${CODEX_NATIVE_RAW_DIR:-$CODEX_NATIVE_NATIVE_ROOT/raw}"
CODEX_NATIVE_RAW_VENDOR="${CODEX_NATIVE_RAW_VENDOR:-$CODEX_NATIVE_RAW_DIR/vendor/aarch64-unknown-linux-musl}"
CODEX_NATIVE_RUNTIME_DIR="${CODEX_NATIVE_RUNTIME_DIR:-$CODEX_NATIVE_NATIVE_ROOT/current}"
CODEX_NATIVE_CURRENT_LINK="${CODEX_NATIVE_CURRENT_LINK:-$CODEX_NATIVE_RUNTIME_DIR}"
CODEX_NATIVE_VERIFIED_LINK="${CODEX_NATIVE_VERIFIED_LINK:-$CODEX_NATIVE_NATIVE_ROOT/verified}"
CODEX_NATIVE_RUNTIME="${CODEX_NATIVE_RUNTIME:-$CODEX_NATIVE_RUNTIME_DIR/codex}"
CODEX_NATIVE_MANAGED_SHELL="${CODEX_NATIVE_MANAGED_SHELL:-$CODEX_NATIVE_MANAGER_DIR/managed.sh}"
CODEX_NATIVE_STATE_DIR="${CODEX_NATIVE_STATE_DIR:-$CODEX_NATIVE_HOME/.local/share/codex/native}"
CODEX_NATIVE_PROFILE_ROOT="${CODEX_NATIVE_PROFILE_ROOT:-$CODEX_NATIVE_HOME/.codex-profiles}"
CODEX_NATIVE_SHARED_PLUGINS_DIR="${CODEX_NATIVE_SHARED_PLUGINS_DIR:-$CODEX_NATIVE_HOME/.codex/plugins}"
CODEX_NATIVE_STATE_FILE="${CODEX_NATIVE_STATE_FILE:-$CODEX_NATIVE_STATE_DIR/state.json}"
CODEX_NATIVE_REGISTRY_FILE="${CODEX_NATIVE_REGISTRY_FILE:-$CODEX_NATIVE_STATE_DIR/registry.json}"
CODEX_NATIVE_STORE_DIR="${CODEX_NATIVE_STORE_DIR:-$CODEX_NATIVE_NATIVE_ROOT/store}"
CODEX_NATIVE_RUNTIME_STORE_DIR="${CODEX_NATIVE_RUNTIME_STORE_DIR:-$CODEX_NATIVE_STORE_DIR/runtime}"
CODEX_NATIVE_RAW_STORE_DIR="${CODEX_NATIVE_RAW_STORE_DIR:-$CODEX_NATIVE_STORE_DIR/raw}"
CODEX_NATIVE_LEGACY_STORE_DIR="${CODEX_NATIVE_LEGACY_STORE_DIR:-$CODEX_NATIVE_STATE_DIR/store}"
CODEX_NATIVE_STORE_MIGRATION_REPORT="${CODEX_NATIVE_STORE_MIGRATION_REPORT:-$CODEX_NATIVE_STATE_DIR/legacy-store-migration.json}"
CODEX_NATIVE_BACKUP_DIR="${CODEX_NATIVE_BACKUP_DIR:-$CODEX_NATIVE_STATE_DIR/backups}"
CODEX_NATIVE_DOCTOR_DIR="${CODEX_NATIVE_DOCTOR_DIR:-$CODEX_NATIVE_STATE_DIR/doctor}"
CODEX_NATIVE_LOCK_FILE="${CODEX_NATIVE_LOCK_FILE:-$CODEX_NATIVE_STATE_DIR/native.lock}"
CODEX_NATIVE_LOCK_WAIT_SECONDS="${CODEX_NATIVE_LOCK_WAIT_SECONDS:-30}"
CODEX_NATIVE_RESOLV_CONF="${CODEX_NATIVE_RESOLV_CONF:-$CODEX_NATIVE_PREFIX/etc/resolv.conf}"
CODEX_NATIVE_CERT_FILE="${CODEX_NATIVE_CERT_FILE:-$CODEX_NATIVE_PREFIX/etc/tls/cert.pem}"
CODEX_NATIVE_CERT_DIR="${CODEX_NATIVE_CERT_DIR:-$CODEX_NATIVE_PREFIX/etc/tls/certs}"
CODEX_NATIVE_RESOLVER_FD="${CODEX_NATIVE_RESOLVER_FD:-33}"
CODEX_NATIVE_PACKAGE_SPEC_DEFAULT="${CODEX_NATIVE_PACKAGE_SPEC_DEFAULT:-@openai/codex@linux-arm64}"
CODEX_NATIVE_MANAGED_LAUNCHER_MARKER="${CODEX_NATIVE_MANAGED_LAUNCHER_MARKER:-codex native managed launcher}"
CODEX_NATIVE_PUBLIC_CODEX="${CODEX_NATIVE_PUBLIC_CODEX:-$CODEX_NATIVE_PREFIX/bin/codex}"
CODEX_NATIVE_RUNTIME_BUILDER="${CODEX_NATIVE_RUNTIME_BUILDER:-$CODEX_NATIVE_MANAGER_DIR/build-runtime.py}"
CODEX_NATIVE_AUTO_UPDATE="${CODEX_NATIVE_AUTO_UPDATE:-1}"
CODEX_NATIVE_AUTO_UPDATE_MODE="${CODEX_NATIVE_AUTO_UPDATE_MODE:-prompt}"
CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS="${CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS:-21600}"
CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS="${CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS:-4}"
CODEX_NATIVE_AUTO_UPDATE_STAMP="${CODEX_NATIVE_AUTO_UPDATE_STAMP:-$CODEX_NATIVE_STATE_DIR/last-auto-update-check}"
CODEX_NATIVE_AUTO_UPDATE_PENDING="${CODEX_NATIVE_AUTO_UPDATE_PENDING:-$CODEX_NATIVE_STATE_DIR/pending-auto-update-version}"
CODEX_NATIVE_AUTO_UPDATE_FAILED="${CODEX_NATIVE_AUTO_UPDATE_FAILED:-$CODEX_NATIVE_STATE_DIR/failed-auto-update}"
CODEX_NATIVE_LAST_PROFILE_FILE="${CODEX_NATIVE_LAST_PROFILE_FILE:-$CODEX_NATIVE_STATE_DIR/last-profile}"
CODEX_NATIVE_RUNTIME_RETENTION="${CODEX_NATIVE_RUNTIME_RETENTION:-3}"
CODEX_NATIVE_PATCH_POLICY="${CODEX_NATIVE_PATCH_POLICY:-dns-fd33-only-v1}"

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
    codex_native_cmd hash-file --path "$1"
}

codex_now() {
    date -Is
}

codex_native_package_root() {
    local source_dir source_root package_root=""
    source_dir="${BASH_SOURCE[0]%/*}"
    [ "$source_dir" = "${BASH_SOURCE[0]}" ] && source_dir="."
    source_dir="$(cd "$source_dir" && pwd)"
    source_root="$(cd "$source_dir/.." && pwd)"
    if [ -d "$source_root/tools/codex_native" ]; then
        package_root="$source_root/tools"
    elif [ -n "${ROOT_DIR:-}" ] && [ -d "$ROOT_DIR/tools/codex_native" ]; then
        package_root="$ROOT_DIR/tools"
    elif [ -d "$CODEX_NATIVE_MANAGER_DIR/codex_native" ]; then
        package_root="$CODEX_NATIVE_MANAGER_DIR"
    else
        codex_fail "internal helper package is unavailable"
        return 1
    fi
    printf '%s\n' "$package_root"
}

codex_native_cmd() {
    local package_root
    package_root="$(codex_native_package_root)" || return 1
    PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" python3 -m codex_native.cli "$@"
}

codex_native_activation_cmd() {
    local action="$1" shell_lib="${BASH_SOURCE[0]}" wrapper_version wrapper_commit
    shift
    wrapper_version="$(codex_current_wrapper_version)" || return 1
    wrapper_commit="$(codex_current_wrapper_commit)" || return 1
    codex_native_cmd "$action" \
        --current-link "$CODEX_NATIVE_CURRENT_LINK" \
        --verified-link "$CODEX_NATIVE_VERIFIED_LINK" \
        --raw-link "$CODEX_NATIVE_RAW_DIR" \
        --state-file "$CODEX_NATIVE_STATE_FILE" \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_NATIVE_RAW_STORE_DIR" \
        --wrapper-version "$wrapper_version" \
        --wrapper-commit "$wrapper_commit" \
        --updated-at "$(codex_now)" \
        --shell-bin "${BASH:-bash}" \
        --shell-lib "$shell_lib" \
        --home "$CODEX_NATIVE_HOME" \
        --prefix "$CODEX_NATIVE_PREFIX" \
        --manager-dir "$CODEX_NATIVE_MANAGER_DIR" \
        --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --resolv-conf "$CODEX_NATIVE_RESOLV_CONF" \
        --cert-file "$CODEX_NATIVE_CERT_FILE" \
        --cert-dir "$CODEX_NATIVE_CERT_DIR" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY" \
        "$@"
}

codex_file_has_marker() {
    local path="$1"
    [ -e "$path" ] || return 1
    grep -a -q "$CODEX_NATIVE_MANAGED_LAUNCHER_MARKER" "$path" 2>/dev/null
}

codex_with_lock() {
    local cmd="$1"
    shift
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    if command -v flock >/dev/null 2>&1; then
        (
            if ! flock -w "$CODEX_NATIVE_LOCK_WAIT_SECONDS" -x 9; then
                codex_fail "another mutation operation is in progress: $CODEX_NATIVE_LOCK_FILE"
                exit 75
            fi
            "$cmd" "$@"
        ) 9>"$CODEX_NATIVE_LOCK_FILE"
    else
        local lock_dir="${CODEX_NATIVE_LOCK_FILE}.d" waited=0
        while ! mkdir "$lock_dir" 2>/dev/null; do
            sleep 1
            waited=$((waited + 1))
            if [ "$waited" -ge "$CODEX_NATIVE_LOCK_WAIT_SECONDS" ]; then
                codex_fail "another mutation operation is in progress: $CODEX_NATIVE_LOCK_FILE"
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
    run_home="${CODEX_NATIVE_HOME:-$HOME}"
    runtime_dir="$(codex_parent_dir "$executable")"
    if [ -d "$CODEX_NATIVE_CERT_DIR" ]; then
        cert_dir_env=("SSL_CERT_DIR=$CODEX_NATIVE_CERT_DIR")
    fi
    runtime_env=(env -u LD_PRELOAD -u LD_LIBRARY_PATH \
        -u CODEX_MANAGED_BY_NPM -u CODEX_MANAGED_BY_BUN -u CODEX_MANAGED_PACKAGE_ROOT \
        HOME="$run_home" \
        XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$run_home/.config}" \
        XDG_CACHE_HOME="${XDG_CACHE_HOME:-$run_home/.cache}" \
        XDG_DATA_HOME="${XDG_DATA_HOME:-$run_home/.local/share}" \
        GODEBUG="${GODEBUG:-netdns=go}" \
        SSL_CERT_FILE="$CODEX_NATIVE_CERT_FILE" \
        CODEX_SELF_EXE="$executable" \
        CODEX_NATIVE_BWRAP_COMPAT_QUIET="${CODEX_NATIVE_BWRAP_COMPAT_QUIET:-1}" \
        PATH="$runtime_dir/codex-path:$runtime_dir/codex-resources:$CODEX_NATIVE_PREFIX/bin:$PATH" \
        "${cert_dir_env[@]}")
    if [ ! -r "$CODEX_NATIVE_RESOLV_CONF" ]; then
        codex_fail "resolver source is unavailable: $CODEX_NATIVE_RESOLV_CONF"
        return 66
    fi
    "${runtime_env[@]}" "$executable" "$@" 33<"$CODEX_NATIVE_RESOLV_CONF"
}

codex_smoke_test_runtime() {
    local executable="$1"
    shift || true
    codex_runtime_exec "$executable" --version "$@" >/dev/null 2>&1
}

codex_validate_tarball_safe() {
    codex_native_cmd validate-tarball --path "$1"
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
    if [ -r "$CODEX_NATIVE_MANAGER_DIR/bwrap-termux-compat.py" ] &&
        [ -r "$CODEX_NATIVE_MANAGER_DIR/rg-termux-shim.sh" ]; then
        printf '%s\n' "$CODEX_NATIVE_MANAGER_DIR"
    elif [ -r "$CODEX_NATIVE_RUNTIME_DIR/bwrap-termux-compat.py" ] &&
        [ -r "$CODEX_NATIVE_RUNTIME_DIR/rg-termux-shim.sh" ]; then
        printf '%s\n' "$CODEX_NATIVE_RUNTIME_DIR"
    else
        printf '%s\n' "$CODEX_NATIVE_MANAGER_DIR"
    fi
}

codex_resolve_path() {
    codex_native_cmd resolve-path --path "$1"
}

codex_tree_digest() {
    codex_native_cmd tree-digest --path "$1"
}

codex_publish_immutable_tree() {
    codex_native_cmd store-publish-tree \
        --source-dir "$1" \
        --target-dir "$2" >/dev/null
}

codex_write_json_state() {
    local version="$1" raw_sha="$2" runtime_sha="$3" package_spec="$4" active_tuple_id="${5:-}"
    local wrapper_version wrapper_commit verified_tuple_id="${6:-}" verified_at="${7:-}"
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    codex_native_cmd state-write \
        --state-file "$CODEX_NATIVE_STATE_FILE" \
        --version "$version" \
        --raw-sha256 "$raw_sha" \
        --runtime-sha256 "$runtime_sha" \
        --package-spec "$package_spec" \
        --active-tuple-id "$active_tuple_id" \
        --wrapper-version "$wrapper_version" \
        --wrapper-commit "$wrapper_commit" \
        --updated-at "$(codex_now)" \
        --verified-tuple-id "$verified_tuple_id" \
        --verified-at "$verified_at"
}

codex_read_state_field() {
    local field="$1"
    codex_native_cmd state-read-field \
        --state-file "$CODEX_NATIVE_STATE_FILE" \
        --field "$field"
}

codex_record_registry() {
    local version="$1" raw_sha="$2" runtime_sha="$3" package_spec="$4" runtime_path="${5:-}" smoke_tested_at="${6:-}" raw_path="${7:-}"
    local wrapper_version wrapper_commit
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    codex_native_cmd registry-record \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --version "$version" \
        --raw-sha256 "$raw_sha" \
        --runtime-sha256 "$runtime_sha" \
        --package-spec "$package_spec" \
        --runtime-path "$runtime_path" \
        --wrapper-version "$wrapper_version" \
        --wrapper-commit "$wrapper_commit" \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --updated-at "$(codex_now)" \
        --smoke-tested-at "$smoke_tested_at" \
        --raw-path "$raw_path"
}

codex_store_id() {
    local version="$1" sha="$2" tree_sha="${3:-}" builder_sha="unknown" bwrap_sha="unknown" rg_sha="unknown" support_dir
    support_dir="$(codex_support_source_dir)"
    if [ -r "$CODEX_NATIVE_RUNTIME_BUILDER" ]; then
        builder_sha="$(codex_sha256 "$CODEX_NATIVE_RUNTIME_BUILDER")"
    fi
    if [ -r "$support_dir/bwrap-termux-compat.py" ]; then
        bwrap_sha="$(codex_sha256 "$support_dir/bwrap-termux-compat.py")"
    fi
    if [ -r "$support_dir/rg-termux-shim.sh" ]; then
        rg_sha="$(codex_sha256 "$support_dir/rg-termux-shim.sh")"
    fi
    codex_native_cmd store-id \
        --version "$version" \
        --sha256 "$sha" \
        --builder-sha256 "$builder_sha" \
        --bwrap-sha256 "$bwrap_sha" \
        --rg-sha256 "$rg_sha" \
        --tree-sha256 "$tree_sha"
}

codex_validate_runtime_retention() {
    case "$CODEX_NATIVE_RUNTIME_RETENTION" in
        ''|*[!0-9]*|0)
            codex_fail "CODEX_NATIVE_RUNTIME_RETENTION must be an integer greater than zero"
            return 2
            ;;
    esac
}

codex_prune_runtime_store() {
    codex_validate_runtime_retention || return $?
    codex_native_cmd store-prune \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_NATIVE_RAW_STORE_DIR" \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --state-file "$CODEX_NATIVE_STATE_FILE" \
        --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY" \
        --retention "$CODEX_NATIVE_RUNTIME_RETENTION" \
        --current-link "$CODEX_NATIVE_RUNTIME_DIR" \
        --verified-link "$CODEX_NATIVE_VERIFIED_LINK" \
        --raw-link "$CODEX_NATIVE_RAW_DIR" >/dev/null
}

codex_store_runtime_payload() {
    local version="$1" runtime_sha="$2" src_runtime_dir="${3:-$CODEX_NATIVE_RUNTIME_DIR}" store_id dst
    local runtime_tree_sha
    runtime_tree_sha="$(codex_tree_digest "$(codex_resolve_path "$src_runtime_dir")")"
    store_id="$(codex_store_id "$version" "$runtime_sha" "$runtime_tree_sha")"
    dst="$CODEX_NATIVE_RUNTIME_STORE_DIR/$store_id"
    codex_native_cmd store-publish-runtime \
        --source-dir "$src_runtime_dir" \
        --target-dir "$dst" \
        --expected-sha256 "$runtime_sha"
}

codex_store_raw_payload() {
    local version="$1" raw_sha="$2" src_raw_dir="${3:-$CODEX_NATIVE_RAW_DIR}" store_id dst
    local raw_tree_sha
    raw_tree_sha="$(codex_tree_digest "$(codex_resolve_path "$src_raw_dir")")"
    store_id="$(codex_store_id "$version" "$raw_sha" "$raw_tree_sha")"
    dst="$CODEX_NATIVE_RAW_STORE_DIR/$store_id"
    codex_native_cmd store-publish-raw \
        --source-dir "$src_raw_dir" \
        --target-dir "$dst" \
        --expected-sha256 "$raw_sha"
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
    [ -x "$CODEX_NATIVE_RUNTIME_BUILDER" ] &&
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
        printf '%s\n' "$CODEX_NATIVE_PACKAGE_SPEC_DEFAULT"
    elif [[ "$requested" == @openai/codex@* ]]; then
        printf '%s\n' "$requested"
    elif [[ "$requested" == *linux-arm64 ]]; then
        printf '@openai/codex@%s\n' "$requested"
    else
        printf '@openai/codex@%s-linux-arm64\n' "$requested"
    fi
}

codex_extract_pack_field() {
    codex_native_cmd package-field --json-file "$1" --field "$2"
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
    local src_vendor="$1" target_dir="${2:-$CODEX_NATIVE_RAW_DIR}" staged
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
    local builder="$CODEX_NATIVE_RUNTIME_BUILDER"
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
    local raw_store_src="$CODEX_NATIVE_RAW_DIR" runtime_target raw_target
    local cleanup_raw=() runtime_tree_sha raw_tree_sha
    if [ -n "$raw_src" ]; then
        raw_store_src="$raw_src"
        cleanup_raw=(--cleanup-raw-source)
    fi
    runtime_tree_sha="$(codex_tree_digest "$(codex_resolve_path "$runtime_src")")"
    raw_tree_sha="$(codex_tree_digest "$(codex_resolve_path "$raw_store_src")")"
    runtime_target="$CODEX_NATIVE_RUNTIME_STORE_DIR/$(codex_store_id "$version" "$runtime_sha" "$runtime_tree_sha")"
    raw_target="$CODEX_NATIVE_RAW_STORE_DIR/$(codex_store_id "$version" "$raw_sha" "$raw_tree_sha")"
    codex_native_activation_cmd activation-commit \
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
    local runtime_stage="$CODEX_NATIVE_RUNTIME_DIR.build.$$" runtime_complete="$CODEX_NATIVE_RUNTIME_DIR.complete.$$"
    [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ] || return 1
    mkdir -p "$CODEX_NATIVE_STATE_DIR" "$CODEX_NATIVE_DOCTOR_DIR"
    report="$CODEX_NATIVE_DOCTOR_DIR/last-build-report.json"
    build_stdout="$CODEX_NATIVE_DOCTOR_DIR/last-build-report.stdout"
    rm -rf "$runtime_stage"
    if [ "${CODEX_NATIVE_BUILD_VERBOSE:-0}" = "1" ]; then
        if ! "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report"; then
            rm -rf "$runtime_stage"
            return 1
        fi
    else
        if ! "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_RAW_VENDOR" --runtime-dir "$runtime_stage" --report-json "$report" >"$build_stdout" 2>&1; then
            rm -rf "$runtime_stage"
            return 1
        fi
    fi
    codex_say "preparing runtime bundle"
    if ! codex_prepare_complete_runtime_tree "$runtime_stage" "$runtime_complete"; then
        rm -rf "$runtime_stage" "$runtime_complete"
        return 1
    fi
    raw_sha="$(codex_sha256 "$CODEX_NATIVE_RAW_VENDOR/bin/codex")"
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
    raw_stage="$CODEX_NATIVE_RAW_DIR.update.$$"
    runtime_stage="$CODEX_NATIVE_RUNTIME_DIR.update.$$"
    runtime_complete="$CODEX_NATIVE_RUNTIME_DIR.complete.$$"
    mkdir -p "$CODEX_NATIVE_DOCTOR_DIR"
    codex_say "staging raw vendor tree"
    if ! codex_install_raw_vendor "$vendor" "$raw_stage"; then
        rm -rf "$tmp"
        return 1
    fi
    codex_say "building patched runtime"
    if ! codex_build_runtime_tree "$raw_stage/vendor/aarch64-unknown-linux-musl" "$runtime_stage" "$CODEX_NATIVE_DOCTOR_DIR/last-build-report.stdout"; then
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
        timeout "$CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | tr -d '"'
    else
        npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | tr -d '"'
    fi
}

codex_auto_update_mode() {
    local mode="${CODEX_NATIVE_AUTO_UPDATE_MODE:-prompt}"
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
    [ "$CODEX_NATIVE_AUTO_UPDATE" = "0" ] && return 1
    [ "$(codex_auto_update_mode)" != "off" ] || return 1
    now="$(date +%s)"
    last="$(cat "$CODEX_NATIVE_AUTO_UPDATE_STAMP" 2>/dev/null || printf '0')"
    [ $((now - last)) -ge "$CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS" ]
}

codex_mark_auto_update_checked() {
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    date +%s >"$CODEX_NATIVE_AUTO_UPDATE_STAMP"
}

codex_read_pending_auto_update() {
    cat "$CODEX_NATIVE_AUTO_UPDATE_PENDING" 2>/dev/null || true
}

codex_write_pending_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    printf '%s\n' "$version" >"$CODEX_NATIVE_AUTO_UPDATE_PENDING"
}

codex_clear_pending_auto_update() {
    rm -f "$CODEX_NATIVE_AUTO_UPDATE_PENDING"
}

codex_read_failed_auto_update() {
    cat "$CODEX_NATIVE_AUTO_UPDATE_FAILED" 2>/dev/null || true
}

codex_write_failed_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    printf '%s\t%s\n' "$version" "$(date +%s)" >"$CODEX_NATIVE_AUTO_UPDATE_FAILED"
}

codex_clear_failed_auto_update() {
    rm -f "$CODEX_NATIVE_AUTO_UPDATE_FAILED"
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
    [ $((now - failed_at)) -ge "$CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS" ]
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
    [ "$CODEX_NATIVE_AUTO_UPDATE" = "0" ] && return 0
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

# shellcheck disable=SC1091
. "$CODEX_NATIVE_SHELL_DIR/codex-termux-runtime.sh"

# shellcheck disable=SC1091
. "$CODEX_NATIVE_SHELL_DIR/codex-termux-interactive.sh"
