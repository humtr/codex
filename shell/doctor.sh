# shellcheck shell=bash
# This file is sourced by ../codex-termux.sh; do not execute directly.

codex_termux_doctor_json() {
    local version raw_sha runtime_sha
    version="$(codex_termux_cmd state-read-field --state-file "$CODEX_TERMUX_STATE_FILE" --field version)"
    raw_sha="$(codex_termux_cmd state-read-field --state-file "$CODEX_TERMUX_STATE_FILE" --field raw_sha256)"
    runtime_sha="$(codex_termux_cmd state-read-field --state-file "$CODEX_TERMUX_STATE_FILE" --field runtime_sha256)"
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

codex_termux_doctor() {
    codex_status_clear
    if [ "${1:-}" = "--json" ]; then
        codex_termux_doctor_json
    else
        codex_termux_doctor_json | codex_termux_cmd doctor-render --mode human
    fi
}



codex_termux_doctor_public() {
    case "${1:-}" in
        ""|--json)
            [ "$#" -le 1 ] || {
                codex_fail "termux doctor accepts only --json"
                return 2
            }
            codex_termux_doctor "$@"
            ;;
        -h|--help|help)
            cat <<'USAGE'
Usage: codex termux doctor [--json]

Runs wrapper-only diagnostics for the managed launcher, runtime resources,
resolver, CA, DNS patch, support state, and registry metadata.
USAGE
            ;;
        *)
            codex_fail "termux doctor accepts only --json"
            return 2
            ;;
    esac
}
