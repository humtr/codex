# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_require_runtime_resolver() {
    if [ ! -r "$CODEX_TERMUX_RESOLV_CONF" ]; then
        codex_fail "Resolver source is unavailable: $CODEX_TERMUX_RESOLV_CONF"
        return 66
    fi
}

codex_runtime_apply_env_plan() {
    local runtime_dir="$1" runtime_exe="$2" set_home="${3:-0}" run_home="${4:-}" termux_open_url="${5:-0}"
    local runtime_env
    runtime_env="$(codex_termux_cmd runtime-env-plan \
        --runtime-dir "$runtime_dir" \
        --runtime-exe "$runtime_exe" \
        --set-home "$set_home" \
        --home "$run_home" \
        --tmpdir "$CODEX_TERMUX_TMPDIR" \
        --cert-file "$CODEX_TERMUX_CERT_FILE" \
        --cert-dir "$CODEX_TERMUX_CERT_DIR" \
        --prefix "$CODEX_TERMUX_PREFIX" \
        --path "$PATH" \
        --browser "${BROWSER:-}" \
        --ssl-cert-file "${SSL_CERT_FILE:-}" \
        --ssl-cert-dir "${SSL_CERT_DIR:-}" \
        --xdg-config-home "${XDG_CONFIG_HOME:-}" \
        --xdg-cache-home "${XDG_CACHE_HOME:-}" \
        --xdg-data-home "${XDG_DATA_HOME:-}" \
        --godebug "${GODEBUG:-}" \
        --bwrap-quiet "${CODEX_TERMUX_BWRAP_COMPAT_QUIET:-}" \
        --termux-open-url "$termux_open_url")" || return $?
    eval "$runtime_env"
}

codex_runtime_exec() {
    local executable="$1"
    shift || true
    local run_home runtime_dir
    run_home="${CODEX_TERMUX_HOME:-$HOME}"
    case "$executable" in
        */*) runtime_dir="${executable%/*}" ;;
        *) runtime_dir="$executable" ;;
    esac
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    codex_runtime_apply_env_plan "$runtime_dir" "$executable" 1 "$run_home" 0 || return $?
    codex_require_runtime_resolver || return $?
    codex_prepare_system_config || return $?
    "$CODEX_SELF_EXE" "$@" 33<"$CODEX_TERMUX_RESOLV_CONF" 34<"$CODEX_TERMUX_SYSTEM_CONFIG_DIR"
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
    codex_termux_cmd support-source-dir \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR"
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
    codex_termux_cmd runtime-retention-ok --value "$CODEX_TERMUX_RUNTIME_RETENTION" || {
        codex_fail "CODEX_TERMUX_RUNTIME_RETENTION must be an integer greater than zero"
        return 2
    }
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
    codex_termux_cmd package-spec \
        --requested "${1:-}" \
        --default "$CODEX_TERMUX_PACKAGE_SPEC_DEFAULT"
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
    local plan_env
    codex_raw_integrity_ok || {
        codex_fail "$(codex_ui_text_get cached_raw_integrity_failed)"
        return 1
    }
    plan_env="$(codex_termux_cmd runtime-cached-build-plan-env \
        --state-file "$CODEX_TERMUX_STATE_FILE")" || return $?
    eval "$plan_env"
    codex_runtime_build_cached_unlocked \
        "$CODEX_RUNTIME_CACHED_VERSION" \
        "$CODEX_RUNTIME_CACHED_PACKAGE_SPEC" || return $?
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

codex_runtime_wrapper_metadata_env() {
    codex_termux_cmd wrapper-metadata-env \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR"
}

codex_repair_diagnose_action() {
    local field="${1:-action}" metadata_env
    metadata_env="$(codex_runtime_wrapper_metadata_env)" || return 1
    eval "$metadata_env"
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
        --wrapper-version "$CODEX_WRAPPER_VERSION" \
        --wrapper-commit "$CODEX_WRAPPER_COMMIT" \
        --field "$field"
}

codex_runtime_action_plan_field() {
    codex_termux_cmd runtime-action-plan \
        --action "$1" \
        --intent "$2" \
        --field "$3"
}

codex_runtime_apply_action() {
    local action="$1" intent="${2:-readiness}" kind step refresh_after error exit_code status=0
    kind="$(codex_runtime_action_plan_field "$action" "$intent" kind)" || return $?
    step="$(codex_runtime_action_plan_field "$action" "$intent" step)" || return $?
    [ -z "$step" ] || codex_ui_step "$step"
    case "$kind" in
        noop)
            return 0
            ;;
        refresh_metadata)
            codex_refresh_runtime_metadata || status=$?
            ;;
        restore_verified)
            codex_try_verified_rollback || status=$?
            ;;
        rebuild_cached)
            codex_runtime_install_cached || status=$?
            ;;
        error)
            error="$(codex_runtime_action_plan_field "$action" "$intent" error)" || return $?
            exit_code="$(codex_runtime_action_plan_field "$action" "$intent" exit-code)" || return $?
            codex_fail "$error"
            return "$exit_code"
            ;;
        *)
            codex_fail "Unknown runtime action executor: $kind"
            return 1
            ;;
    esac
    [ "$status" -eq 0 ] || return "$status"
    refresh_after="$(codex_runtime_action_plan_field "$action" "$intent" refresh-after)" || return $?
    [ "$refresh_after" != "1" ] || codex_refresh_runtime_metadata
}

codex_repair_apply() {
    local action support_attempted=0
    while :; do
        action="$(codex_repair_diagnose_action action)" || return $?
        case "$action" in
            refresh_support)
                if [ "$support_attempted" = "1" ]; then
                    codex_fail "Support layer repair did not complete; run bash install.sh from a wrapper checkout"
                    return 1
                fi
                support_attempted=1
                codex_repair_install_support || return $?
                ;;
            *)
                codex_runtime_apply_action "$action" repair
                return $?
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

codex_mark_auto_update_checked() {
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    date +%s >"$CODEX_TERMUX_AUTO_UPDATE_STAMP"
}

codex_write_pending_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\n' "$version" >"$CODEX_TERMUX_AUTO_UPDATE_PENDING"
}

codex_clear_pending_auto_update() {
    rm -f "$CODEX_TERMUX_AUTO_UPDATE_PENDING"
}

codex_write_failed_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_TERMUX_STATE_DIR"
    printf '%s\t%s\n' "$version" "$(date +%s)" >"$CODEX_TERMUX_AUTO_UPDATE_FAILED"
}

codex_clear_failed_auto_update() {
    rm -f "$CODEX_TERMUX_AUTO_UPDATE_FAILED"
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
    choice="$(codex_termux_cmd update-prompt-decision --choice "${CODEX_PROMPT_CHOICE_RESULT:-}")" || return $?
    case "$choice" in
        apply)
            return 0
            ;;
        keep)
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
    local current latest pending plan_env last now failed
    codex_runtime_ok || return 0
    current="$(codex_read_state_field version)"
    pending="$(cat "$CODEX_TERMUX_AUTO_UPDATE_PENDING" 2>/dev/null || true)"
    last="$(cat "$CODEX_TERMUX_AUTO_UPDATE_STAMP" 2>/dev/null || printf '0')"
    now="$(date +%s)"
    plan_env="$(codex_termux_cmd auto-update-check-plan-env \
        --enabled "$CODEX_TERMUX_AUTO_UPDATE" \
        --mode "${CODEX_TERMUX_AUTO_UPDATE_MODE:-prompt}" \
        --current "$current" \
        --pending "$pending" \
        --now "$now" \
        --last "$last" \
        --interval "$CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS")" || return $?
    eval "$plan_env"
    [ "$CODEX_AUTO_UPDATE_CLEAR_PENDING" = "0" ] || codex_clear_pending_auto_update
    case "$CODEX_AUTO_UPDATE_ACTION" in
        skip)
            return 0
            ;;
        use_pending)
            latest="$CODEX_AUTO_UPDATE_LATEST"
            ;;
        fetch)
            codex_mark_auto_update_checked
            latest="$(codex_latest_linux_arm64_version || true)"
            if [ -z "$latest" ]; then
                [ -z "$pending" ] || codex_clear_pending_auto_update
                return 0
            fi
            ;;
        *)
            return 0
            ;;
    esac
    if [ "$latest" != "$current" ]; then
        codex_write_pending_auto_update "$latest"
        failed="$(cat "$CODEX_TERMUX_AUTO_UPDATE_FAILED" 2>/dev/null || true)"
        now="$(date +%s)"
        plan_env="$(codex_termux_cmd auto-update-apply-plan-env \
            --current "$current" \
            --latest "$latest" \
            --failed-record "$failed" \
            --mode "$CODEX_AUTO_UPDATE_MODE" \
            --now "$now" \
            --interval "$CODEX_TERMUX_AUTO_UPDATE_INTERVAL_SECONDS")" || return $?
        eval "$plan_env"
        case "$CODEX_AUTO_UPDATE_ACTION" in
            install)
                codex_install_auto_update "$current" "$latest" || return 0
                ;;
            prompt)
                codex_prompt_update "$current" "$latest"
                case "$?" in
                    0)
                        codex_install_auto_update "$current" "$latest" || return 0
                        ;;
                    130)
                        return 130
                        ;;
                esac
                ;;
        esac
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
    local metadata_env
    metadata_env="$(codex_runtime_wrapper_metadata_env)" || return 1
    eval "$metadata_env"
    codex_termux_cmd runtime-metadata-current \
        --state-path "$CODEX_TERMUX_STATE_FILE" \
        --registry-path "$CODEX_TERMUX_REGISTRY_FILE" \
        --current "$CODEX_TERMUX_RUNTIME_DIR" \
        --verified "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw "$CODEX_TERMUX_RAW_DIR" \
        --wrapper-version "$CODEX_WRAPPER_VERSION" \
        --wrapper-commit "$CODEX_WRAPPER_COMMIT"
}

codex_refresh_runtime_metadata_unlocked() {
    local metadata_current=0 plan_env
    codex_runtime_metadata_current && metadata_current=1
    plan_env="$(codex_termux_cmd runtime-refresh-plan-env \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --metadata-current "$metadata_current")" || return $?
    eval "$plan_env"
    [ "$CODEX_RUNTIME_REFRESH_ACTION" = "activate" ] || return 0
    codex_activate_tuple_unlocked \
        "$CODEX_TERMUX_RUNTIME_DIR" \
        "$CODEX_RUNTIME_REFRESH_VERSION" \
        "$CODEX_RUNTIME_REFRESH_RAW_SHA256" \
        "$CODEX_RUNTIME_REFRESH_RUNTIME_SHA256" \
        "$CODEX_RUNTIME_REFRESH_PACKAGE_SPEC"
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
    codex_runtime_apply_action "$action" readiness
}


codex_prepare_runtime_env() {
    local runtime_dir runtime_exe termux_open_url=0
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    codex_prepare_system_config || return $?
    runtime_dir="$(codex_resolve_path "$CODEX_TERMUX_RUNTIME_DIR")" || return $?
    runtime_exe="$runtime_dir/codex"
    command -v termux-open-url >/dev/null 2>&1 && termux_open_url=1
    codex_runtime_apply_env_plan "$runtime_dir" "$runtime_exe" 0 "" "$termux_open_url"
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

codex_current_runtime_date() {
    codex_termux_cmd registry-active-runtime-date \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE"
}

codex_display_dotted_date() {
    codex_termux_cmd display-runtime-date --value "${1:-}"
}

codex_read_upstream_release_date_cache() {
    codex_termux_cmd upstream-release-cache-read \
        --cache "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" \
        --version "$1"
}

codex_write_upstream_release_date_cache() {
    codex_termux_cmd upstream-release-cache-write \
        --cache "$CODEX_TERMUX_UPSTREAM_TIME_CACHE" \
        --version "$1" \
        --release-date "$2" >/dev/null 2>&1 || true
}

codex_fetch_upstream_release_date() {
    local version="${1:-}"
    [ -n "$version" ] || return 0
    if command -v timeout >/dev/null 2>&1; then
        timeout "$CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex time --json 2>/dev/null | \
            codex_termux_cmd upstream-release-date --version "$version"
    else
        npm view @openai/codex time --json 2>/dev/null | \
            codex_termux_cmd upstream-release-date --version "$version"
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
    local upstream upstream_version upstream_date runtime_date metadata_env status=0
    codex_status_clear
    if upstream="$(codex_run_current_runtime --version 2>/dev/null)"; then
        status=0
    else
        status=$?
        upstream=""
    fi
    upstream_version="$(codex_termux_cmd upstream-version --text "$upstream")"
    upstream_date="$(codex_upstream_release_date "$upstream_version" || true)"
    runtime_date="$(codex_display_dotted_date "$(codex_current_runtime_date || true)")"
    metadata_env="$(codex_runtime_wrapper_metadata_env)" || return 1
    eval "$metadata_env"
    codex_termux_cmd version-report \
        --upstream "$upstream" \
        --upstream-date "$upstream_date" \
        --runtime-date "$runtime_date" \
        --wrapper-version "$CODEX_WRAPPER_VERSION" \
        --wrapper-commit "$CODEX_WRAPPER_COMMIT" >&2
    return "$status"
}
