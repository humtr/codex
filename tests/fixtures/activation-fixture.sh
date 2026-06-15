#!/usr/bin/env bash

activation_fixture_export_env() {
    local root="$1" manager="$root/native/manager"
    mkdir -p "$manager" "$root/pointers/current" "$root/pointers/verified" \
        "$root/pointers/raw" "$root/metadata/state" "$root/metadata/registry" \
        "$root/native/store/runtime" "$root/native/store/raw" "$root/markers"
    cp "$ROOT_DIR/tools/build-runtime.py" "$manager/build-runtime.py"
    cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager/bwrap-termux-compat.py"
    cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager/rg-termux-shim.sh"
    printf '#!/bin/sh\nexit 0\n' >"$manager/managed.sh"
    printf 'support-lib\n' >"$manager/lib.sh"
    printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' \
        >"$manager/wrapper-version.env"
    chmod 755 "$manager/managed.sh" "$manager/build-runtime.py" \
        "$manager/bwrap-termux-compat.py" "$manager/rg-termux-shim.sh"
    printf 'nameserver 127.0.0.1\n' >"$root/resolv.conf"
    printf 'cert\n' >"$root/cert.pem"

    export CODEX_NATIVE_HOME="$root/home"
    export CODEX_NATIVE_NATIVE_ROOT="$root/native"
    export CODEX_NATIVE_MANAGER_DIR="$manager"
    export CODEX_NATIVE_RUNTIME_DIR="$root/pointers/current/current"
    export CODEX_NATIVE_CURRENT_LINK="$CODEX_NATIVE_RUNTIME_DIR"
    export CODEX_NATIVE_RUNTIME="$CODEX_NATIVE_RUNTIME_DIR/codex"
    export CODEX_NATIVE_VERIFIED_LINK="$root/pointers/verified/verified"
    export CODEX_NATIVE_RAW_DIR="$root/pointers/raw/raw"
    export CODEX_NATIVE_RAW_VENDOR="$CODEX_NATIVE_RAW_DIR/vendor/aarch64-unknown-linux-musl"
    export CODEX_NATIVE_STATE_DIR="$root/metadata/state"
    export CODEX_NATIVE_STATE_FILE="$CODEX_NATIVE_STATE_DIR/state.json"
    export CODEX_NATIVE_REGISTRY_FILE="$root/metadata/registry/registry.json"
    export CODEX_NATIVE_STORE_DIR="$root/native/store"
    export CODEX_NATIVE_RUNTIME_STORE_DIR="$CODEX_NATIVE_STORE_DIR/runtime"
    export CODEX_NATIVE_RAW_STORE_DIR="$CODEX_NATIVE_STORE_DIR/raw"
    export CODEX_NATIVE_RUNTIME_BUILDER="$manager/build-runtime.py"
    export CODEX_NATIVE_RESOLV_CONF="$root/resolv.conf"
    export CODEX_NATIVE_CERT_FILE="$root/cert.pem"
    export CODEX_NATIVE_RUNTIME_RETENTION=20
}

activation_fixture_make_candidate() {
    local root="$1" label="$2" mode="${3:-healthy}"
    local runtime="$root/candidate.$label.runtime" raw="$root/candidate.$label.raw"
    local raw_vendor="$raw/vendor/aarch64-unknown-linux-musl" builder_sha
    rm -rf "$runtime" "$raw"
    mkdir -p "$runtime/codex-resources/zsh/bin" "$runtime/codex-path" "$raw_vendor/bin"
    activation_fixture_write_executable "$runtime/codex" "$root" "$label" "$mode"
    printf '#!/bin/sh\nexit 0\n' >"$runtime/codex-resources/bwrap"
    printf '#!/bin/sh\nexit 0\n' >"$runtime/codex-resources/zsh/bin/zsh"
    cp "$CODEX_NATIVE_MANAGER_DIR/bwrap-termux-compat.py" "$runtime/codex-path/bwrap"
    cp "$CODEX_NATIVE_MANAGER_DIR/rg-termux-shim.sh" "$runtime/codex-path/rg"
    printf '#!/bin/sh\nexit 0\n' >"$runtime/codex-path/rg.real"
    printf '{}\n' >"$runtime/codex-package.json"
    printf 'raw %s\n' "$label" >"$raw_vendor/bin/codex"
    chmod 755 "$runtime/codex" "$runtime/codex-resources/bwrap" \
        "$runtime/codex-resources/zsh/bin/zsh" "$runtime/codex-path/bwrap" \
        "$runtime/codex-path/rg" "$runtime/codex-path/rg.real" "$raw_vendor/bin/codex"
    ACTIVATION_RUNTIME_SHA="$(sha256sum "$runtime/codex" | awk '{print $1}')"
    ACTIVATION_RAW_SHA="$(sha256sum "$raw_vendor/bin/codex" | awk '{print $1}')"
    builder_sha="$(sha256sum "$CODEX_NATIVE_RUNTIME_BUILDER" | awk '{print $1}')"
    printf '{"builder_sha256":"%s","patch_policy":"dns-fd33-only-v1","raw_sha256":"%s","runtime_sha256":"%s"}\n' \
        "$builder_sha" "$ACTIVATION_RAW_SHA" "$ACTIVATION_RUNTIME_SHA" >"$runtime/runtime-build.json"
    ACTIVATION_RUNTIME="$runtime"
    ACTIVATION_RAW="$raw"
}

activation_fixture_write_executable() {
    local path="$1" root="$2" label="$3" mode="$4" marker="$root/markers/$label"
    case "$mode" in
        healthy)
            printf '#!/bin/sh\n[ "${1:-}" = "--version" ] && printf "codex %s\\n"\nexit 0\n' \
                "$label" >"$path"
            ;;
        fail-readiness|fail-cleanup|fail-state-restore)
            cat >"$path" <<EOF
#!/bin/sh
marker="$marker"
if [ -e "\$marker" ]; then
    [ "$mode" != "fail-cleanup" ] || chmod a-w "\$CODEX_NATIVE_RUNTIME_STORE_DIR"
    [ "$mode" != "fail-state-restore" ] || chmod a-w "\$CODEX_NATIVE_STATE_DIR"
    exit 42
fi
: >"\$marker"
exit 0
EOF
            ;;
        *)
            printf 'activation fixture: unknown executable mode: %s\n' "$mode" >&2
            return 1
            ;;
    esac
}

activation_fixture_activate_candidate() {
    local version="$1"
    codex_activate_tuple_unlocked \
        "$ACTIVATION_RUNTIME" "$version" "$ACTIVATION_RAW_SHA" \
        "$ACTIVATION_RUNTIME_SHA" "@openai/codex@$version" "$ACTIVATION_RAW"
}

activation_fixture_capture() {
    local root="$1" expected="$root/expected"
    rm -rf "$expected"
    mkdir -p "$expected"
    readlink "$CODEX_NATIVE_CURRENT_LINK" >"$expected/current"
    readlink "$CODEX_NATIVE_VERIFIED_LINK" >"$expected/verified"
    readlink "$CODEX_NATIVE_RAW_DIR" >"$expected/raw"
    cp "$CODEX_NATIVE_STATE_FILE" "$expected/state.json"
    cp "$CODEX_NATIVE_REGISTRY_FILE" "$expected/registry.json"
}

activation_fixture_assert_unchanged() {
    local root="$1" expected="$root/expected"
    [ "$(readlink "$CODEX_NATIVE_CURRENT_LINK")" = "$(cat "$expected/current")" ]
    [ "$(readlink "$CODEX_NATIVE_VERIFIED_LINK")" = "$(cat "$expected/verified")" ]
    [ "$(readlink "$CODEX_NATIVE_RAW_DIR")" = "$(cat "$expected/raw")" ]
    cmp -s "$CODEX_NATIVE_STATE_FILE" "$expected/state.json"
    cmp -s "$CODEX_NATIVE_REGISTRY_FILE" "$expected/registry.json"
}
