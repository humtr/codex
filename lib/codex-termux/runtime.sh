# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_require_runtime_resolver() {
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "Resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
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
        codex_fail "Cached raw package integrity check failed; run codex termux update"
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

codex_repair_install_support() {
    local source
    source="$(codex_install_source_command)" || {
        codex_fail "Install source is unavailable; run bash install.sh from a wrapper checkout"
        return 1
    }
    codex_ui_step repair_support
    CODEX_TERMUX_INSTALL_PRINT_VERSION=0 CODEX_TERMUX_INSTALL_SURFACE=0 bash "$source" install support >/dev/null
}

codex_repair_diagnose_action() {
    local field="${1:-action}" wrapper_version wrapper_commit
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    codex_termux_cmd repair-diagnose \
        --managed-shell "$CODEX_TERMUX_MANAGED_SHELL" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --public-codex "$CODEX_TERMUX_PUBLIC_CODEX" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --support-dir "$(codex_support_source_dir)" \
        --manifest-path "$CODEX_TERMUX_RUNTIME_DIR/runtime-build.json" \
        --builder "$CODEX_TERMUX_RUNTIME_BUILDER" \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --raw-binary "$CODEX_TERMUX_RAW_VENDOR/bin/codex" \
        --patch-policy "$CODEX_TERMUX_PATCH_POLICY" \
        --wrapper-version "$wrapper_version" \
        --wrapper-commit "$wrapper_commit" \
        --field "$field"
}

codex_repair_apply() {
    local action support_attempted=0
    while :; do
        action="$(codex_repair_diagnose_action action)" || return $?
        case "$action" in
            none)
                return 0
                ;;
            refresh_support)
                if [ "$support_attempted" = "1" ]; then
                    codex_fail "Support layer repair did not complete; run bash install.sh from a wrapper checkout"
                    return 1
                fi
                support_attempted=1
                codex_repair_install_support || return $?
                ;;
            refresh_metadata)
                codex_ui_step repair_metadata
                codex_refresh_runtime_metadata
                return $?
                ;;
            restore_verified)
                codex_try_verified_rollback || return $?
                codex_refresh_runtime_metadata
                return $?
                ;;
            rebuild_cached)
                codex_ui_step repair_runtime
                codex_runtime_install_cached || return $?
                codex_refresh_runtime_metadata
                return $?
                ;;
            unrecoverable)
                codex_fail "Runtime is damaged and cached raw is unavailable or invalid; run codex termux update"
                return 1
                ;;
            *)
                codex_fail "Unknown repair action: $action"
                return 1
                ;;
        esac
    done
}

codex_repair_core_unlocked() {
    codex_validate_runtime_retention || return $?
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
    codex_termux_cmd runtime-layout-ok \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR" \
        --runtime "$CODEX_TERMUX_RUNTIME" \
        --support-dir "$(codex_support_source_dir)"
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
    codex_support_tools_match &&
    [ -r "$CODEX_TERMUX_STATE_FILE" ] &&
    codex_runtime_integrity_ok
}

codex_support_layer_ok() {
    codex_termux_cmd support-layer-ok \
        --managed-shell "$CODEX_TERMUX_MANAGED_SHELL" \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --public-codex "$CODEX_TERMUX_PUBLIC_CODEX" \
        --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER"
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
    local action
    action="$(codex_repair_diagnose_action readiness-action)" || return $?
    case "$action" in
        ready)
            return 0
            ;;
        refresh_metadata)
            codex_refresh_runtime_metadata
            return $?
            ;;
        restore_verified)
            codex_try_verified_rollback || return $?
            codex_refresh_runtime_metadata
            return $?
            ;;
        rebuild_cached)
            codex_ui_step rebuild_cached_runtime
            codex_runtime_install_cached
            return $?
            ;;
        raw_corrupt)
            codex_fail "Cached raw package integrity check failed; run codex termux update"
            return 1
            ;;
        missing_runtime)
            codex_fail "Runtime is missing and no cached raw package is available; run codex termux update"
            return 127
            ;;
        *)
            codex_fail "Unknown runtime readiness action: $action"
            return 1
            ;;
    esac
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
