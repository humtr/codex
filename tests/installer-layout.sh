#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_PARENT="${TMPDIR:-/data/data/com.termux/files/usr/tmp}"

fail() {
    printf 'installer-layout: FAIL: %s\n' "$*" >&2
    exit 1
}

mkdir -p "$FIXTURE_PARENT"

reset_native_env() {
    local var
    while IFS= read -r var; do
        unset "$var"
    done < <(compgen -A variable CODEX_NATIVE_)
}

assert_manager_python_package() {
    local manager_dir="$1"
    [ -r "$manager_dir/codex_native/__init__.py" ] || fail "manager missing codex_native package"
    [ -r "$manager_dir/codex_native/cli.py" ] || fail "manager missing codex_native cli"
    [ -r "$manager_dir/codex_native/errors.py" ] || fail "manager missing codex_native errors"
    [ -r "$manager_dir/codex_native/hashing.py" ] || fail "manager missing codex_native hashing"
    [ -r "$manager_dir/codex_native/store.py" ] || fail "manager missing codex_native store"
    PYTHONPATH="$manager_dir" python3 -m codex_native.cli validate >/dev/null \
        || fail "manager codex_native package did not validate"
}

run_support_case() {
    local fixture_root prefix native_root state_root manager_dir external_bwrap_before sentinel
    fixture_root="$(mktemp -d "$FIXTURE_PARENT/installer-layout-support.XXXXXX")"
    prefix="$fixture_root/prefix"
    native_root="$fixture_root/home/.local/lib/codex/native"
    state_root="$fixture_root/home/.local/share/codex/native"
    manager_dir="$native_root/manager"
    reset_native_env
    export CODEX_NATIVE_HOME="$fixture_root/home"
    export PREFIX="$prefix"
    export CODEX_NATIVE_NATIVE_ROOT="$native_root"
    export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
    export CODEX_NATIVE_STATE_DIR="$state_root"
    export CODEX_NATIVE_STATE_FILE="$state_root/state.json"
    export CODEX_NATIVE_REGISTRY_FILE="$state_root/registry.json"
    export CODEX_NATIVE_STORE_DIR="$native_root/store"
    export CODEX_NATIVE_RUNTIME_DIR="$native_root/current"
    export CODEX_NATIVE_CURRENT_LINK="$native_root/current"
    export CODEX_NATIVE_VERIFIED_LINK="$native_root/verified"
    export CODEX_NATIVE_RAW_DIR="$native_root/raw"
    export CODEX_NATIVE_PUBLIC_CODEX="$prefix/bin/codex"

    mkdir -p "$prefix/bin"
    printf '#!/bin/sh\nprintf external-bwrap\\n\n' >"$prefix/bin/bwrap"
    chmod 755 "$prefix/bin/bwrap"
    external_bwrap_before="$(sha256sum "$prefix/bin/bwrap")"

    # shellcheck disable=SC1091
    . "$ROOT_DIR/bin/install-runtime.sh"

    mkdir -p "$CODEX_NATIVE_MANAGER_DIR" "$CODEX_NATIVE_STATE_DIR"
    sentinel="$fixture_root/support-sentinel"
    codex_launcher_available() { return 1; }
    npm() { printf 'npm\n' >>"$sentinel"; return 96; }
    curl() { printf 'curl\n' >>"$sentinel"; return 95; }

    main support

    [ -x "$manager_dir/managed.sh" ] || fail "support did not install managed shell"
    [ -r "$manager_dir/lib.sh" ] || fail "support did not install manager lib"
    [ -x "$manager_dir/build-runtime.py" ] || fail "support did not install runtime builder"
    [ -x "$manager_dir/bwrap-termux-compat.py" ] || fail "support did not install bwrap compat"
    [ -x "$manager_dir/rg-termux-shim.sh" ] || fail "support did not install rg shim"
    [ -r "$manager_dir/codex-termux-runtime.sh" ] || fail "support did not install runtime shell lib"
    [ -r "$manager_dir/codex-termux-interactive.sh" ] || fail "support did not install interactive shell lib"
    [ -r "$manager_dir/wrapper-version.env" ] || fail "support did not install wrapper version metadata"
    assert_manager_python_package "$manager_dir"
    grep -Fq ". \"$manager_dir/lib.sh\"" "$manager_dir/managed.sh" \
        || fail "managed shell does not source manager lib"
    grep -Fq "codex-termux-runtime.sh" "$manager_dir/lib.sh" \
        || fail "manager lib does not source runtime shell lib"
    grep -Fq "codex-termux-interactive.sh" "$manager_dir/lib.sh" \
        || fail "manager lib does not source interactive shell lib"
    grep -Fq "$CODEX_NATIVE_MANAGED_LAUNCHER_MARKER" "$CODEX_NATIVE_PUBLIC_CODEX" \
        || fail "public launcher missing managed marker"
    grep -Fq "exec \"$manager_dir/managed.sh\" \"\$@\"" "$CODEX_NATIVE_PUBLIC_CODEX" \
        || fail "public launcher does not target manager managed shell"
    [ ! -e "$CODEX_NATIVE_RUNTIME_DIR" ] || fail "support created current pointer"
    [ ! -e "$CODEX_NATIVE_VERIFIED_LINK" ] || fail "support created verified pointer"
    [ ! -e "$CODEX_NATIVE_RAW_DIR" ] || fail "support created raw pointer"
    [ ! -e "$CODEX_NATIVE_STORE_DIR" ] || fail "support created store layout"
    [ ! -e "$sentinel" ] || fail "support touched fetch/update/repair/network paths"
    [ "$(sha256sum "$prefix/bin/bwrap")" = "$external_bwrap_before" ] \
        || fail "support changed public bwrap"
    rm -rf "$fixture_root"
}

build_legacy_runtime_fixture() {
    local fixture_root="$1" native_root="$2" state_root="$3"
    local manager_dir="$native_root/manager" legacy_runtime="$native_root/runtime"
    local legacy_raw="$native_root/raw/vendor/aarch64-unknown-linux-musl"
    local runtime_sha raw_sha builder_sha

    mkdir -p "$manager_dir" "$legacy_runtime/codex-resources/zsh/bin" "$legacy_runtime/codex-path" \
        "$legacy_raw/bin" "$state_root"
    cp "$ROOT_DIR/tools/build-runtime.py" "$manager_dir/build-runtime.py"
    cp "$ROOT_DIR/tools/bwrap-termux-compat.py" "$manager_dir/bwrap-termux-compat.py"
    cp "$ROOT_DIR/tools/rg-termux-shim.sh" "$manager_dir/rg-termux-shim.sh"
    printf 'CODEX_NATIVE_WRAPPER_VERSION=test\nCODEX_NATIVE_WRAPPER_COMMIT=test\n' >"$manager_dir/wrapper-version.env"
    chmod 755 "$manager_dir/build-runtime.py" "$manager_dir/bwrap-termux-compat.py" "$manager_dir/rg-termux-shim.sh"

    cat >"$legacy_runtime/codex" <<'EOF'
#!/bin/sh
[ "${1:-}" = "--version" ] && printf 'codex legacy\n'
exit 0
EOF
    printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-resources/bwrap"
    printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-resources/zsh/bin/zsh"
    cp "$manager_dir/bwrap-termux-compat.py" "$legacy_runtime/codex-path/bwrap"
    cp "$manager_dir/rg-termux-shim.sh" "$legacy_runtime/codex-path/rg"
    printf '#!/bin/sh\nexit 0\n' >"$legacy_runtime/codex-path/rg.real"
    printf '{}\n' >"$legacy_runtime/codex-package.json"
    printf 'raw legacy\n' >"$legacy_raw/bin/codex"
    chmod 755 "$legacy_runtime/codex" "$legacy_runtime/codex-resources/bwrap" \
        "$legacy_runtime/codex-resources/zsh/bin/zsh" "$legacy_runtime/codex-path/bwrap" \
        "$legacy_runtime/codex-path/rg" "$legacy_runtime/codex-path/rg.real" "$legacy_raw/bin/codex"

    runtime_sha="$(sha256sum "$legacy_runtime/codex" | awk '{print $1}')"
    raw_sha="$(sha256sum "$legacy_raw/bin/codex" | awk '{print $1}')"
    builder_sha="$(sha256sum "$manager_dir/build-runtime.py" | awk '{print $1}')"
    cat >"$legacy_runtime/runtime-build.json" <<EOF
{"patch_policy":"dns-fd33-only-v1","builder_sha256":"$builder_sha","runtime_sha256":"$runtime_sha","raw_sha256":"$raw_sha"}
EOF
    cat >"$state_root/state.json" <<EOF
{"version":"legacy","raw_sha256":"$raw_sha","runtime_sha256":"$runtime_sha","package_spec":"@openai/codex@legacy"}
EOF
}

run_setup_case() {
    local fixture_root prefix native_root state_root manager_dir sentinel
    fixture_root="$(mktemp -d "$FIXTURE_PARENT/installer-layout-setup.XXXXXX")"
    prefix="$fixture_root/prefix"
    native_root="$fixture_root/native"
    state_root="$fixture_root/state"
    manager_dir="$native_root/manager"
    reset_native_env
    build_legacy_runtime_fixture "$fixture_root" "$native_root" "$state_root"
    printf 'nameserver 127.0.0.1\n' >"$fixture_root/resolv.conf"
    printf 'cert\n' >"$fixture_root/cert.pem"

    export CODEX_NATIVE_HOME="$fixture_root/home"
    export PREFIX="$prefix"
    export CODEX_NATIVE_NATIVE_ROOT="$native_root"
    export CODEX_NATIVE_MANAGER_DIR="$manager_dir"
    export CODEX_NATIVE_RUNTIME_DIR="$native_root/current"
    export CODEX_NATIVE_RUNTIME="$native_root/current/codex"
    export CODEX_NATIVE_CURRENT_LINK="$native_root/current"
    export CODEX_NATIVE_VERIFIED_LINK="$native_root/verified"
    export CODEX_NATIVE_STATE_DIR="$state_root"
    export CODEX_NATIVE_STATE_FILE="$state_root/state.json"
    export CODEX_NATIVE_REGISTRY_FILE="$state_root/registry.json"
    export CODEX_NATIVE_STORE_DIR="$native_root/store"
    export CODEX_NATIVE_RUNTIME_BUILDER="$manager_dir/build-runtime.py"
    export CODEX_NATIVE_RESOLV_CONF="$fixture_root/resolv.conf"
    export CODEX_NATIVE_CERT_FILE="$fixture_root/cert.pem"
    export CODEX_NATIVE_PUBLIC_CODEX="$prefix/bin/codex"

    mkdir -p "$prefix/bin"

    # shellcheck disable=SC1091
    . "$ROOT_DIR/bin/install-runtime.sh"

    mkdir -p "$CODEX_NATIVE_MANAGER_DIR" "$CODEX_NATIVE_STATE_DIR"
    sentinel="$fixture_root/setup-sentinel"
    codex_launcher_available() { return 1; }
    npm() { printf 'npm\n' >>"$sentinel"; return 96; }
    curl() { printf 'curl\n' >>"$sentinel"; return 95; }

    main setup

    [ ! -e "$sentinel" ] || fail "setup touched fetch/update/repair/network paths"
    [ -x "$manager_dir/managed.sh" ] || fail "setup did not install manager support"
    assert_manager_python_package "$manager_dir"
    grep -Fq "exec \"$manager_dir/managed.sh\" \"\$@\"" "$CODEX_NATIVE_PUBLIC_CODEX" \
        || fail "setup launcher does not target manager managed shell"
    [ -L "$CODEX_NATIVE_RUNTIME_DIR" ] || fail "setup did not create current pointer"
    [ -L "$CODEX_NATIVE_VERIFIED_LINK" ] || fail "setup did not create verified pointer"
    [ -L "$CODEX_NATIVE_RAW_DIR" ] || fail "setup did not create raw pointer"
    case "$(readlink "$CODEX_NATIVE_RUNTIME_DIR")" in
        "$CODEX_NATIVE_RUNTIME_STORE_DIR"/*) ;;
        *) fail "current pointer does not target runtime store child" ;;
    esac
    case "$(readlink "$CODEX_NATIVE_VERIFIED_LINK")" in
        "$CODEX_NATIVE_RUNTIME_STORE_DIR"/*) ;;
        *) fail "verified pointer does not target runtime store child" ;;
    esac
    case "$(readlink "$CODEX_NATIVE_RAW_DIR")" in
        "$CODEX_NATIVE_RAW_STORE_DIR"/*) ;;
        *) fail "raw pointer does not target raw store child" ;;
    esac
    [ "$(readlink "$CODEX_NATIVE_RUNTIME_DIR")" = "$(readlink "$CODEX_NATIVE_VERIFIED_LINK")" ] \
        || fail "current and verified pointers differ after setup migration"
    codex_runtime_ok || fail "setup-migrated runtime is not ready"
    [ -d "$native_root/runtime" ] || fail "setup removed legacy runtime directory"

    python3 -c 'import json, sys; from pathlib import Path; registry_path, state_path, current, verified, raw = map(Path, sys.argv[1:6]); registry = json.loads(registry_path.read_text()); state = json.loads(state_path.read_text()); active = registry["active_tuple_id"]; verified_tuple = state["verified_tuple_id"]; assert active == state["active_tuple_id"]; assert active == verified_tuple; runtime_entry = registry["runtime"][active]; raw_entry = registry["raw"][runtime_entry["raw_id"]]; assert Path(runtime_entry["path"]).resolve() == current.resolve(); assert Path(registry["runtime"][verified_tuple]["path"]).resolve() == verified.resolve(); assert Path(raw_entry["path"]).resolve() == raw.resolve()' \
        "$CODEX_NATIVE_REGISTRY_FILE" "$CODEX_NATIVE_STATE_FILE" \
        "$CODEX_NATIVE_RUNTIME_DIR" "$CODEX_NATIVE_VERIFIED_LINK" "$CODEX_NATIVE_RAW_DIR"
    rm -rf "$fixture_root"
}

run_compiled_launcher_case() {
    local fixture_root binary output
    fixture_root="$(mktemp -d "$FIXTURE_PARENT/installer-layout-launcher.XXXXXX")"
    grep -Fq '.local/lib/codex/native/manager/managed.sh' "$ROOT_DIR/tools/codex-launcher.c" \
        || fail "compiled launcher source is missing manager target"
    if grep -Fq '.local/lib/codex/native/runtime/managed.sh' "$ROOT_DIR/tools/codex-launcher.c"; then
        fail "compiled launcher source still references legacy runtime target"
    fi

    command -v clang >/dev/null 2>&1 || return 0

    binary="$fixture_root/codex-launcher"
    clang -O2 -Wall -Wextra -o "$binary" "$ROOT_DIR/tools/codex-launcher.c"
    mkdir -p "$fixture_root/home/.local/lib/codex/native/manager" \
        "$fixture_root/home/.local/lib/codex/native/runtime" "$fixture_root/prefix/bin"
    cat >"$fixture_root/home/.local/lib/codex/native/manager/managed.sh" <<'EOF'
#!/bin/sh
printf 'manager:%s\n' "$*"
EOF
    cat >"$fixture_root/home/.local/lib/codex/native/runtime/managed.sh" <<'EOF'
#!/bin/sh
printf 'legacy\n' >&2
exit 88
EOF
    chmod 755 "$fixture_root/home/.local/lib/codex/native/manager/managed.sh" \
        "$fixture_root/home/.local/lib/codex/native/runtime/managed.sh"
    output="$(
        HOME="$fixture_root/home" \
        PREFIX="$fixture_root/prefix" \
        CODEX_NATIVE_BASH="$(command -v bash)" \
        "$binary" alpha beta
    )"
    [ "$output" = "manager:alpha beta" ] || fail "compiled launcher did not execute manager shell by default"
    rm -rf "$fixture_root"
}

run_support_case
run_setup_case
run_compiled_launcher_case

printf 'installer-layout: ok\n'
