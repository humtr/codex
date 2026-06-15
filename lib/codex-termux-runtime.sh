codex_support_tools_match() {
    local support_dir
    support_dir="$(codex_support_source_dir)"
    cmp -s "$support_dir/bwrap-termux-compat.py" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" &&
    cmp -s "$support_dir/rg-termux-shim.sh" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg"
}

codex_runtime_integrity_ok() {
    [ -r "$CODEX_NATIVE_RUNTIME_DIR/runtime-build.json" ] || return 1
    [ -x "$CODEX_NATIVE_RUNTIME_BUILDER" ] || return 1
    codex_native_cmd runtime-integrity \
        --runtime "$CODEX_NATIVE_RUNTIME" \
        --manifest-path "$CODEX_NATIVE_RUNTIME_DIR/runtime-build.json" \
        --builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --state-path "$CODEX_NATIVE_STATE_FILE" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY"
}

codex_raw_integrity_ok() {
    [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ] || return 1
    codex_native_cmd raw-integrity \
        --raw-binary "$CODEX_NATIVE_RAW_VENDOR/bin/codex" \
        --state-path "$CODEX_NATIVE_STATE_FILE"
}

codex_runtime_ok() {
    [ -x "$CODEX_NATIVE_RUNTIME" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-resources/bwrap" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg.real" ] &&
    codex_support_tools_match &&
    [ -r "$CODEX_NATIVE_STATE_FILE" ] &&
    codex_runtime_integrity_ok
}

codex_runtime_metadata_current() {
    local wrapper_version wrapper_commit
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    codex_native_cmd runtime-metadata-current \
        --state-path "$CODEX_NATIVE_STATE_FILE" \
        --registry-path "$CODEX_NATIVE_REGISTRY_FILE" \
        --current "$CODEX_NATIVE_RUNTIME_DIR" \
        --verified "$CODEX_NATIVE_VERIFIED_LINK" \
        --raw "$CODEX_NATIVE_RAW_DIR" \
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
        "$CODEX_NATIVE_RUNTIME_DIR" "$version" "$raw_sha" "$runtime_sha" "$package_spec"
}

codex_refresh_runtime_metadata() {
    codex_runtime_metadata_current && return 0
    codex_with_lock codex_refresh_runtime_metadata_unlocked
}

codex_activate_cached_runtime_unlocked() {
    local runtime_path="$1" raw_path="$2" version="$3" raw_sha="$4" runtime_sha="$5" package_spec="$6"
    local runtime_complete="$CODEX_NATIVE_RUNTIME_DIR.use.$$" raw_complete="$CODEX_NATIVE_RAW_DIR.use.$$"
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

codex_bootstrap_store() {
    local version raw_sha runtime_sha package_spec runtime_path raw_path tuple_id now
    [ -n "$(codex_read_state_field active_tuple_id)" ] && return 0
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    package_spec="$(codex_read_state_field package_spec)"
    [ -n "$version" ] && [ -n "$runtime_sha" ] && [ -x "$CODEX_NATIVE_RUNTIME" ] || return 0
    codex_smoke_test_runtime "$CODEX_NATIVE_RUNTIME" || return 1
    runtime_path="$(codex_store_runtime_payload "$version" "$runtime_sha")"
    raw_path="$(codex_store_raw_payload "$version" "$raw_sha")" || return 1
    now="$(codex_now)"
    tuple_id="$(codex_record_registry "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$runtime_path" "$now" "$raw_path")"
    codex_write_json_state "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$tuple_id" "$tuple_id" "$now"
}

codex_registry_tuple_for_runtime_path() {
    local runtime_path="$1"
    codex_native_cmd registry-tuple-for-runtime-path \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --runtime-path "$runtime_path"
}

codex_try_verified_rollback_unlocked() {
    codex_native_activation_cmd activation-restore-verified >/dev/null || return 1
    codex_say "active runtime restored from verified tuple"
}

codex_verified_rollback_needed() {
    [ -e "$CODEX_NATIVE_VERIFIED_LINK" ] || [ -L "$CODEX_NATIVE_VERIFIED_LINK" ] || return 1
    [ -x "$(codex_resolve_path "$CODEX_NATIVE_VERIFIED_LINK")/codex" ] || return 1
    [ "$(codex_resolve_path "$CODEX_NATIVE_RUNTIME_DIR")" != "$(codex_resolve_path "$CODEX_NATIVE_VERIFIED_LINK")" ]
}

codex_try_verified_rollback() {
    codex_verified_rollback_needed || return 1
    codex_with_lock codex_try_verified_rollback_unlocked
}

codex_migrate_legacy_store_cache_unlocked() {
    [ "$(codex_resolve_path "$CODEX_NATIVE_LEGACY_STORE_DIR")" != "$(codex_resolve_path "$CODEX_NATIVE_STORE_DIR")" ] || return 0
    [ -d "$CODEX_NATIVE_LEGACY_STORE_DIR/runtime" ] || return 0
    [ -r "$CODEX_NATIVE_REGISTRY_FILE" ] || return 0
    [ -r "$CODEX_NATIVE_STORE_MIGRATION_REPORT" ] && return 0
    mkdir -p "$CODEX_NATIVE_STATE_DIR" "$CODEX_NATIVE_RUNTIME_STORE_DIR" "$CODEX_NATIVE_RAW_STORE_DIR"
    codex_native_cmd legacy-store-migrate \
        --legacy-store-dir "$CODEX_NATIVE_LEGACY_STORE_DIR" \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_NATIVE_RAW_STORE_DIR" \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --manager-dir "$CODEX_NATIVE_MANAGER_DIR" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY" \
        --report-file "$CODEX_NATIVE_STORE_MIGRATION_REPORT" \
        --completed-at "$(codex_now)" >/dev/null
}

codex_legacy_store_migration_needed() {
    [ "$(codex_resolve_path "$CODEX_NATIVE_LEGACY_STORE_DIR")" != "$(codex_resolve_path "$CODEX_NATIVE_STORE_DIR")" ] &&
    [ -d "$CODEX_NATIVE_LEGACY_STORE_DIR/runtime" ] &&
    [ -r "$CODEX_NATIVE_REGISTRY_FILE" ] &&
    [ ! -r "$CODEX_NATIVE_STORE_MIGRATION_REPORT" ]
}

codex_legacy_runtime_layout_migration_needed() {
    [ ! -e "$CODEX_NATIVE_RUNTIME_DIR" ] &&
    [ ! -L "$CODEX_NATIVE_RUNTIME_DIR" ] &&
    [ -x "$CODEX_NATIVE_LEGACY_RUNTIME_DIR/codex" ]
}

codex_migrate_legacy_runtime_layout_unlocked() {
    local version raw_sha runtime_sha package_spec runtime_complete raw_src
    codex_migrate_legacy_store_cache_unlocked
    if [ -e "$CODEX_NATIVE_RUNTIME_DIR" ] || [ -L "$CODEX_NATIVE_RUNTIME_DIR" ]; then
        return 0
    fi
    [ -x "$CODEX_NATIVE_LEGACY_RUNTIME_DIR/codex" ] || return 0
    version="$(codex_read_state_field version 2>/dev/null || true)"
    raw_sha="$(codex_read_state_field raw_sha256 2>/dev/null || true)"
    package_spec="$(codex_read_state_field package_spec 2>/dev/null || true)"
    [ -n "$version" ] || version="unknown"
    [ -n "$package_spec" ] || package_spec="local"
    raw_src="$CODEX_NATIVE_LEGACY_RAW_DIR"
    [ -x "$raw_src/vendor/aarch64-unknown-linux-musl/bin/codex" ] || raw_src="$CODEX_NATIVE_RAW_DIR"
    [ -x "$raw_src/vendor/aarch64-unknown-linux-musl/bin/codex" ] || return 0
    [ -n "$raw_sha" ] || raw_sha="$(codex_sha256 "$raw_src/vendor/aarch64-unknown-linux-musl/bin/codex")"
    runtime_complete="$CODEX_NATIVE_RUNTIME_DIR.migrate.$$"
    if ! codex_prepare_complete_runtime_tree "$CODEX_NATIVE_LEGACY_RUNTIME_DIR" "$runtime_complete"; then
        rm -rf "$runtime_complete"
        return 1
    fi
    runtime_sha="$(codex_sha256 "$runtime_complete/codex")"
    if ! codex_activate_tuple_unlocked "$runtime_complete" "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$raw_src"; then
        rm -rf "$runtime_complete"
        return 1
    fi
    codex_say "migrated legacy runtime layout to current/verified pointers"
}

codex_migrate_legacy_runtime_layout() {
    if ! codex_legacy_store_migration_needed && ! codex_legacy_runtime_layout_migration_needed; then
        return 0
    fi
    codex_with_lock codex_migrate_legacy_runtime_layout_unlocked
}

codex_ensure_runtime_ready() {
    codex_migrate_legacy_runtime_layout || return $?
    if codex_runtime_ok; then
        codex_refresh_runtime_metadata
        return 0
    fi
    if codex_try_verified_rollback; then
        codex_refresh_runtime_metadata
        return 0
    fi
    if [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ]; then
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

codex_detect_upstream_commands() {
    "$CODEX_NATIVE_RUNTIME" --help 2>/dev/null | codex_native_cmd parse-upstream-commands
}

codex_is_upstream_command() {
    local first="$1"
    [ -n "$first" ] || return 1
    codex_detect_upstream_commands | grep -Fxq "$first"
}

codex_open_fd33_and_exec() {
    local args=() first="${1:-}"
    if [ ! -x "$CODEX_NATIVE_RUNTIME" ]; then
        codex_fail "runtime missing; run codex setup"
        return 127
    fi
    codex_prepare_runtime_env
    if [ "$first" = "--" ]; then
        shift
        exec "$CODEX_NATIVE_RUNTIME" "$@"
    fi
    if [ $# -gt 0 ]; then
        if [[ "$first" == -* ]] || codex_is_upstream_command "$first"; then
            args=("$@")
        else
            args=("exec" "$@")
        fi
    fi
    exec "$CODEX_NATIVE_RUNTIME" "${args[@]}"
}

codex_prepare_runtime_env() {
    export SSL_CERT_FILE="${SSL_CERT_FILE:-$CODEX_NATIVE_CERT_FILE}"
    [ -d "$CODEX_NATIVE_CERT_DIR" ] && export SSL_CERT_DIR="${SSL_CERT_DIR:-$CODEX_NATIVE_CERT_DIR}"
    if [ -z "${BROWSER:-}" ] && command -v termux-open-url >/dev/null 2>&1; then
        export BROWSER=termux-open-url
    fi
    export CODEX_SELF_EXE="${CODEX_SELF_EXE:-$CODEX_NATIVE_RUNTIME}"
    unset CODEX_MANAGED_BY_NPM CODEX_MANAGED_BY_BUN CODEX_MANAGED_PACKAGE_ROOT LD_PRELOAD LD_LIBRARY_PATH
    export CODEX_NATIVE_BWRAP_COMPAT_QUIET="${CODEX_NATIVE_BWRAP_COMPAT_QUIET:-1}"
    export PATH="$CODEX_NATIVE_RUNTIME_DIR/codex-path:$CODEX_NATIVE_RUNTIME_DIR/codex-resources:$CODEX_NATIVE_PREFIX/bin:$PATH"
    if [ -r "$CODEX_NATIVE_RESOLV_CONF" ]; then
        eval "exec ${CODEX_NATIVE_RESOLVER_FD}<\"\$CODEX_NATIVE_RESOLV_CONF\""
    fi
}

codex_current_wrapper_version() {
    if [ -f "$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    elif [ -f "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env"
    fi
    printf '%s\n' "${CODEX_NATIVE_WRAPPER_VERSION:-unknown}"
}

codex_current_wrapper_commit() {
    if [ -f "$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_NATIVE_MANAGER_DIR/wrapper-version.env"
    elif [ -f "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env"
    fi
    printf '%s\n' "${CODEX_NATIVE_WRAPPER_COMMIT:-unknown}"
}

codex_version() {
    local upstream wrapper commit status=0
    if upstream="$("$CODEX_NATIVE_RUNTIME" --version 2>/dev/null)"; then
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
    printf '  %-8s  %s\n' '--' 'Force exact upstream passthrough, e.g. `codex -- doctor --json`.'
}

codex_help() {
    if [ -x "$CODEX_NATIVE_RUNTIME" ]; then
        codex_prepare_runtime_env
        "$CODEX_NATIVE_RUNTIME" --help
    fi
    codex_wrapper_help
}

codex_network_boundary_json() {
    local baseline off on reset baseline_status=0 off_status=0 on_status=0 reset_status=0
    local package_root pythonpath
    package_root="$(codex_native_package_root)" || return 1
    pythonpath="$package_root${PYTHONPATH:+:$PYTHONPATH}"
    baseline="$(codex_native_cmd doctor-socket-probe 2>/dev/null)" || baseline_status=$?
    off="$(PYTHONPATH="$pythonpath" "$CODEX_NATIVE_RUNTIME" sandbox \
        -c sandbox_workspace_write.network_access=false \
        python3 -m codex_native.cli doctor-socket-probe 2>/dev/null)" || off_status=$?
    on="$(PYTHONPATH="$pythonpath" "$CODEX_NATIVE_RUNTIME" sandbox \
        -c permissions.wrapper-network.network.enabled=true -P wrapper-network \
        python3 -m codex_native.cli doctor-socket-probe 2>/dev/null)" || on_status=$?
    reset="$(PYTHONPATH="$pythonpath" "$CODEX_NATIVE_RUNTIME" sandbox \
        -c sandbox_workspace_write.network_access=false \
        python3 -m codex_native.cli doctor-socket-probe 2>/dev/null)" || reset_status=$?
    codex_native_cmd doctor-network-boundary \
        --baseline-json "${baseline:-\{\}}" \
        --off-json "${off:-\{\}}" \
        --on-json "${on:-\{\}}" \
        --reset-json "${reset:-\{\}}" \
        --baseline-exit "$baseline_status" \
        --off-exit "$off_status" \
        --on-exit "$on_status" \
        --reset-exit "$reset_status"
}

codex_wrapper_doctor_json() {
    local version raw_sha runtime_sha network_json
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    codex_prepare_runtime_env
    network_json="$(codex_network_boundary_json)"
    codex_native_cmd doctor-report \
        --runtime "$CODEX_NATIVE_RUNTIME" \
        --current-link "$CODEX_NATIVE_RUNTIME_DIR" \
        --verified-link "$CODEX_NATIVE_VERIFIED_LINK" \
        --raw-link "$CODEX_NATIVE_RAW_DIR" \
        --manager-dir "$CODEX_NATIVE_MANAGER_DIR" \
        --runtime-store-dir "$CODEX_NATIVE_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_NATIVE_RAW_STORE_DIR" \
        --raw-vendor "$CODEX_NATIVE_RAW_VENDOR" \
        --resolv-conf "$CODEX_NATIVE_RESOLV_CONF" \
        --cert-file "$CODEX_NATIVE_CERT_FILE" \
        --state-file "$CODEX_NATIVE_STATE_FILE" \
        --registry-file "$CODEX_NATIVE_REGISTRY_FILE" \
        --migration-report-file "$CODEX_NATIVE_STORE_MIGRATION_REPORT" \
        --legacy-store-dir "$CODEX_NATIVE_LEGACY_STORE_DIR" \
        --version "$version" \
        --raw-sha256 "$raw_sha" \
        --runtime-sha256 "$runtime_sha" \
        --prefix "$CODEX_NATIVE_PREFIX" \
        --runtime-builder "$CODEX_NATIVE_RUNTIME_BUILDER" \
        --patch-policy "$CODEX_NATIVE_PATCH_POLICY" \
        --network-json "$network_json"
}

codex_wrapper_doctor() {
    if [ "${1:-}" = "--json" ]; then
        codex_wrapper_doctor_json
    else
        codex_wrapper_doctor_json | codex_native_cmd doctor-render --mode human
    fi
}

codex_public_doctor() {
    if [ $# -gt 0 ]; then
        codex_ensure_runtime_ready || return $?
        codex_prepare_runtime_env
        "$CODEX_NATIVE_RUNTIME" doctor "$@"
        return $?
    fi
    local upstream_status=0 wrapper_status=0
    codex_ensure_runtime_ready || return $?
    codex_prepare_runtime_env
    "$CODEX_NATIVE_RUNTIME" doctor || upstream_status=$?
    printf '\n%s\n\n' '─────────────────────────────────────────────────────────────'
    codex_wrapper_doctor || wrapper_status=$?
    [ "$upstream_status" -eq 0 ] && [ "$wrapper_status" -eq 0 ]
}
