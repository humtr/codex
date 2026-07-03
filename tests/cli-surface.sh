#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$TMP_PARENT/codex-cli-surface-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"
LIB_SH="$ROOT_DIR/lib/codex-termux.sh"
INSTALL_RUNTIME="$ROOT_DIR/bin/install-runtime.sh"

fail() {
    printf 'cli-surface: FAIL: %s\n' "$*" >&2
    exit 1
}

output="$(
    CODEX_TERMUX_HOME="$TMP_DIR/home" \
    CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
    CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
    bash -lc '. "$1"; codex_termux_help' _ "$LIB_SH" 2>&1
)"
case "$output" in
    *"codex termux <command> [args...]"*) ;;
    *) fail 'termux help did not describe the wrapper namespace' ;;
esac
case "$output" in
    *"Top-level codex arguments are reserved for upstream Codex."*) ;;
    *) fail 'termux help did not reserve top-level codex args for upstream' ;;
esac

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; VERSION_CALLED=0; codex_version() { VERSION_CALLED=1; }; codex_termux_main version >/dev/null 2>&1; [ "$VERSION_CALLED" -eq 1 ]; VERSION_CALLED=0; ! codex_termux_main version junk >/dev/null 2>&1; [ "$VERSION_CALLED" -eq 0 ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; codex_ensure_runtime_ready() { return 0; }; codex_auto_update_if_needed() { return 0; }; codex_runtime_exec_with_context() { printf "%s\n" "$*"; }; [ "$(codex_main version)" = "version" ]; [ "$(codex_main doctor --json)" = "doctor --json" ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; codex_install_source_command() { return 1; }; REPAIR_CALLED=0; codex_repair_public() { REPAIR_CALLED=1; }; codex_termux_main repair >/dev/null 2>&1; [ "$REPAIR_CALLED" -eq 1 ]; REPAIR_CALLED=0; ! codex_termux_main repair junk >/dev/null 2>&1; [ "$REPAIR_CALLED" -eq 0 ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; output="$(codex_termux_main repair --help 2>&1)"; case "$output" in *"Codex Termux wrapper commands"*) ;; *) exit 1 ;; esac' _ "$LIB_SH" "$INSTALL_RUNTIME"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; [ "$(codex_display_version 0.142.4-linux-arm64)" = "0.142.4" ]; [ "$(codex_ui_number 3)" = " 3." ]; [ "$(codex_ui_badge current)" = " 🟢 current " ]; sep="$(codex_termux_cmd ui-format --kind separator --value 4)"; [ "$sep" = "────" ]; [ "$(codex_termux_cmd ui-status-text --message Loading)" = "Loading..." ]; [ "$(codex_termux_cmd ui-status-text --message Ready...)" = "Ready..." ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
CODEX_TERMUX_AUTO_UPDATE_TIMEOUT_SECONDS=1 \
bash -lc '. "$1"; timeout() { shift; "$@"; }; npm() { [ "$1" = view ] || exit 21; [ "$2" = "@openai/codex" ] || exit 22; [ "$3" = dist-tags.linux-arm64 ] || exit 23; [ "$4" = --json ] || exit 24; printf "\"0.142.4\"\n"; }; [ "$(codex_latest_linux_arm64_version)" = "0.142.4" ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
TEST_TMP_DIR="$TMP_DIR" \
bash -lc '. "$1"; marker_file="$TEST_TMP_DIR/launcher.bin"; printf "prefix\0%s\0suffix\n" "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER" >"$marker_file"; codex_termux_cmd file-has-marker --path "$marker_file" --marker "$CODEX_TERMUX_MANAGED_LAUNCHER_MARKER"; ! codex_termux_cmd file-has-marker --path "$marker_file" --marker missing' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; ROOT_DIR="$(cd "$(dirname "$1")/.." && pwd)"; stable="$CODEX_TERMUX_RUNTIME_STORE_DIR/stable-runtime"; mkdir -p "$CODEX_TERMUX_CERT_DIR"; termux-open-url() { :; }; codex_prepare_system_config() { return 0; }; codex_termux_cmd() { if [ "$1" = resolve-path ] && [ "$3" = "$CODEX_TERMUX_RUNTIME_DIR" ]; then printf "%s\n" "$stable"; return 0; fi; PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools${PYTHONPATH:+:$PYTHONPATH}:$CODEX_TERMUX_MANAGER_DIR" python3 -B -m codex_termux.cli "$@"; }; export CODEX_SELF_EXE="$CODEX_TERMUX_RUNTIME" CODEX_MANAGED_BY_NPM=1 LD_PRELOAD=bad BROWSER=""; codex_prepare_runtime_env; [ "$CODEX_SELF_EXE" = "$stable/codex" ]; [ "$TMPDIR" = "$CODEX_TERMUX_TMPDIR" ]; [ "$SQLITE_TMPDIR" = "$CODEX_TERMUX_TMPDIR" ]; [ "$SSL_CERT_FILE" = "$CODEX_TERMUX_CERT_FILE" ]; [ "$SSL_CERT_DIR" = "$CODEX_TERMUX_CERT_DIR" ]; [ "$BROWSER" = "termux-open-url" ]; [ -z "${CODEX_MANAGED_BY_NPM+x}" ]; [ -z "${LD_PRELOAD+x}" ]; case "$PATH" in "$stable/codex-path:$stable/codex-resources:"*) ;; *) exit 1 ;; esac' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
bash -lc '. "$1"; stable="$CODEX_TERMUX_RUNTIME_STORE_DIR/stable-runtime"; mkdir -p "$stable" "$CODEX_TERMUX_CERT_DIR" "$(dirname "$CODEX_TERMUX_RESOLV_CONF")" "$CODEX_TERMUX_SYSTEM_CONFIG_DIR"; printf "nameserver 1.1.1.1\n" >"$CODEX_TERMUX_RESOLV_CONF"; cat >"$stable/codex" <<'"'"'SCRIPT'"'"'
#!/bin/sh
expected_home="$1"
expected_cert_file="$2"
expected_cert_dir="$3"
[ "$HOME" = "$expected_home" ] || exit 11
[ "$XDG_CONFIG_HOME" = "$expected_home/.config" ] || exit 12
[ "$XDG_CACHE_HOME" = "$expected_home/.cache" ] || exit 13
[ "$XDG_DATA_HOME" = "$expected_home/.local/share" ] || exit 14
[ "$GODEBUG" = "netdns=go" ] || exit 15
[ "$SSL_CERT_FILE" = "$expected_cert_file" ] || exit 16
[ "$SSL_CERT_DIR" = "$expected_cert_dir" ] || exit 17
[ "$CODEX_SELF_EXE" = "$0" ] || exit 18
[ -z "${CODEX_MANAGED_BY_NPM+x}" ] || exit 19
[ -z "${LD_PRELOAD+x}" ] || exit 20
printf "runtime-env-ok\n"
SCRIPT
chmod 755 "$stable/codex"; export CODEX_MANAGED_BY_NPM=1 LD_PRELOAD=bad; codex_prepare_system_config() { return 0; }; [ "$(codex_runtime_exec "$stable/codex" "$CODEX_TERMUX_HOME" "$CODEX_TERMUX_CERT_FILE" "$CODEX_TERMUX_CERT_DIR")" = "runtime-env-ok" ]' _ "$LIB_SH"

CODEX_TERMUX_HOME="$TMP_DIR/home" \
CODEX_TERMUX_STATE_DIR="$TMP_DIR/state" \
CODEX_TERMUX_TMPDIR="$TMP_DIR/tmp" \
TEST_TMP_DIR="$TMP_DIR" \
bash -lc '. "$1"; source_file="$TEST_TMP_DIR/source/bin/install-runtime.sh"; mkdir -p "$(dirname "$source_file")"; printf "#!/bin/sh\nexit 0\n" >"$source_file"; chmod 755 "$source_file"; ran=""; bash() { ran="$1"; [ "$1" != "$source_file" ]; }; codex_mktemp_dir() { d="$TEST_TMP_DIR/snapshot"; rm -rf "$d"; mkdir -p "$d"; printf "%s\n" "$d"; }; codex_run_install_source_command "$source_file" install; case "$ran" in "$TEST_TMP_DIR/snapshot/source/bin/install-runtime.sh") ;; *) exit 1 ;; esac' _ "$LIB_SH"

printf 'cli-surface: ok\n'
