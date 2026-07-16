codex_load_wrapper_source_config() {
    [ -r "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG" ] || return 0
    # shellcheck disable=SC1090
    . "$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG"
    codex_normalize_wrapper_source_config
}

codex_normalize_wrapper_source_config() {
    local exports
    exports="$(wrapper_cmd wrapper-source-env \
        --repo "${CODEX_TERMUX_WRAPPER_REPO:-}" \
        --ref "${CODEX_TERMUX_WRAPPER_REF:-}" \
        --token "${CODEX_TERMUX_WRAPPER_TOKEN:-}" \
        --git-repo "${CODEX_TERMUX_WRAPPER_GIT_REPO:-}" \
        --git-ref "${CODEX_TERMUX_WRAPPER_GIT_REF:-}" \
        --git-token "${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}" \
        --release-token "${CODEX_TERMUX_WRAPPER_RELEASE_TOKEN:-}")" || return $?
    [ -z "$exports" ] || eval "$exports"
}

codex_wrapper_auth_token() {
    wrapper_cmd wrapper-auth-token \
        --token "${CODEX_TERMUX_WRAPPER_TOKEN:-}" \
        --git-token "${CODEX_TERMUX_WRAPPER_GIT_TOKEN:-}" \
        --release-token "${CODEX_TERMUX_WRAPPER_RELEASE_TOKEN:-}" \
        --github-token "${GITHUB_TOKEN:-}" \
        --allow-gh 1
}

codex_now() {
    date -Is
}

wrapper_package_root() {
    local source_root="$CODEX_TERMUX_WRAPPER_ROOT" root_dir="${ROOT_DIR:-}" candidate
    for candidate in \
        "$source_root/src" \
        "${root_dir:+$root_dir/src}" \
        "$source_root/tools" \
        "${root_dir:+$root_dir/tools}" \
        "$CODEX_TERMUX_MANAGER_DIR/src" \
        "$CODEX_TERMUX_MANAGER_DIR"
    do
        [ -n "$candidate" ] || continue
        if [ -f "$candidate/wrapper/__init__.py" ] || [ -f "$candidate/codex_termux/cli.py" ]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    codex_fail "Internal helper package is unavailable"
    return 1
}

codex_termux_package_root() {
    wrapper_package_root
}

wrapper_cmd() {
    local package_root module
    package_root="$(wrapper_package_root)" || return 1
    if [ -f "$package_root/wrapper/__init__.py" ]; then
        module="wrapper.cli"
    else
        module="codex_termux.cli"
    fi
    CODEX_TERMUX_HOME="$CODEX_TERMUX_HOME" \
    CODEX_TERMUX_PROFILE_ROOT="$CODEX_TERMUX_PROFILE_ROOT" \
    CODEX_TERMUX_STATE_DIR="$CODEX_TERMUX_STATE_DIR" \
    CODEX_TERMUX_LAST_PROFILE_FILE="$CODEX_TERMUX_LAST_PROFILE_FILE" \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONPATH="$package_root${PYTHONPATH:+:$PYTHONPATH}" \
        python3 -B -m "$module" "$@"
}

codex_termux_cmd() {
    wrapper_cmd "$@"
}

codex_termux_activation_cmd() {
    local action="$1" shell_lib="$CODEX_TERMUX_SHELL_LIB" metadata_env
    shift
    metadata_env="$(wrapper_cmd wrapper-metadata-env \
        --manager-dir "$CODEX_TERMUX_MANAGER_DIR" \
        --runtime-dir "$CODEX_TERMUX_RUNTIME_DIR")" || return 1
    eval "$metadata_env"
    wrapper_cmd "$action" \
        --current-link "$CODEX_TERMUX_CURRENT_LINK" \
        --verified-link "$CODEX_TERMUX_VERIFIED_LINK" \
        --raw-link "$CODEX_TERMUX_RAW_DIR" \
        --state-file "$CODEX_TERMUX_STATE_FILE" \
        --registry-file "$CODEX_TERMUX_REGISTRY_FILE" \
        --runtime-store-dir "$CODEX_TERMUX_RUNTIME_STORE_DIR" \
        --raw-store-dir "$CODEX_TERMUX_RAW_STORE_DIR" \
        --wrapper-version "$CODEX_WRAPPER_VERSION" \
        --wrapper-commit "$CODEX_WRAPPER_COMMIT" \
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
