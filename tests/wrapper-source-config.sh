#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${PREFIX:-/data/data/com.termux/files/usr}/tmp}"
TMP_DIR="$TMP_PARENT/codex-wrapper-source-config-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'wrapper-source-config: FAIL: %s\n' "$*" >&2
    exit 1
}

file_mode() {
    stat -c '%a' "$1" 2>/dev/null || stat -f '%Lp' "$1"
}

CONFIG="$TMP_DIR/wrapper-source.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG" \
    CODEX_TERMUX_WRAPPER_GIT_TOKEN="github_pat_test" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private dev >/dev/null \
    || fail 'install.sh source failed'

[ -f "$CONFIG" ] || fail 'source config was not created'
[ "$(file_mode "$CONFIG")" = "600" ] || fail "source config mode is not 600: $(file_mode "$CONFIG")"

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_TOKEN" = "github_pat_test" ]
' _ "$CONFIG" || fail 'source config did not restore saved values'

CODEX_TERMUX_WRAPPER_GIT_REPO="override/repo" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "override/repo" ]
' _ "$CONFIG" || fail 'explicit environment did not override saved repo'

CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_TOKEN" = "github_pat_test" ]
' _ "$ROOT_DIR/bin/install-runtime.sh" || fail 'install-runtime did not auto-load source config'

CONFIG_DEFAULT_REF="$TMP_DIR/wrapper-source-default-ref.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_DEFAULT_REF" \
    CODEX_TERMUX_WRAPPER_GIT_TOKEN="github_pat_test" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private "" >/dev/null \
    || fail 'install.sh source with empty ref failed'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "main" ]
' _ "$CONFIG_DEFAULT_REF" || fail 'empty source ref did not default to main'

CONFIG_REUSE="$TMP_DIR/wrapper-source-reuse.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_REUSE" \
    CODEX_TERMUX_WRAPPER_GIT_TOKEN="saved_pat_test" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private main >/dev/null \
    || fail 'initial source config for token reuse failed'

CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_REUSE" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private dev </dev/null >/dev/null \
    || fail 'source config did not reuse saved token'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_TOKEN" = "saved_pat_test" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "dev" ]
' _ "$CONFIG_REUSE" || fail 'source config token reuse wrote wrong values'

bash -lc '
    . "$1"
    CONFIGURE_ARGS=""
    INSTALL_CALLED=0
    configure_wrapper_source() { CONFIGURE_ARGS="$*"; }
    install_managed_runtime() { INSTALL_CALLED=1; [ "$#" -eq 0 ]; }
    main source example/private dev
    [ "$CONFIGURE_ARGS" = "example/private dev" ] &&
    [ "$INSTALL_CALLED" -eq 1 ]
' _ "$ROOT_DIR/install.sh" || fail 'source command did not continue into install by default'

bash -lc '
    . "$1"
    INSTALL_CALLED=0
    configure_wrapper_source() { return 0; }
    install_managed_runtime() { INSTALL_CALLED=1; }
    main source --save-only example/private dev
    [ "$INSTALL_CALLED" -eq 0 ]
' _ "$ROOT_DIR/install.sh" || fail 'source --save-only continued into install'

bash -lc '
    . "$1"
    PREFIX="$2/prefix"
    mkdir -p "$PREFIX/bin"
    printf "#!/bin/sh\nexit 0\n" >"$PREFIX/bin/pkg"
    printf "#!/bin/sh\nexit 0\n" >"$PREFIX/bin/codex"
    chmod 755 "$PREFIX/bin/pkg" "$PREFIX/bin/codex"
    CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$2/wrapper-source.env"
    printf "%s\n" \
        "CODEX_TERMUX_WRAPPER_GIT_REPO=example/private" \
        "CODEX_TERMUX_WRAPPER_RELEASE_REPO=example/private" \
        >"$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG"
    install_dependencies() { return 0; }
    bash() {
        case "$1 $2" in
            "$ROOT_DIR/bin/install-runtime.sh install")
                [ "${CODEX_TERMUX_WRAPPER_SOURCE_DIR:-}" = "$ROOT_DIR" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_GIT_URL:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_GIT_REPO:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_RELEASE_URL:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_RELEASE_REPO:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_RELEASE_TAG:-}" ]
                return
                ;;
            "$ROOT_DIR/bin/install-runtime.sh doctor")
                return 0
                ;;
        esac
        command bash "$@"
    }
    CODEX_TERMUX_INSTALL_VERSION_OUTPUT=0 install_managed_runtime
' _ "$ROOT_DIR/install.sh" "$TMP_DIR" || fail 'top-level install did not force local wrapper source'

CONFIG_BOOTSTRAP="$TMP_DIR/wrapper-source-bootstrap.env"
bash -lc '
    . "$1"
    CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$2"
    CODEX_TERMUX_BOOTSTRAPPED=1
    CODEX_TERMUX_BOOTSTRAP_REPO=example/private
    CODEX_TERMUX_BOOTSTRAP_REF=dev
    CODEX_TERMUX_WRAPPER_GIT_TOKEN=github_pat_bootstrap
    save_current_source_config_if_missing
' _ "$ROOT_DIR/install.sh" "$CONFIG_BOOTSTRAP" || fail 'bootstrap source config was not saved'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_GIT_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_GIT_TOKEN" = "github_pat_bootstrap" ]
' _ "$CONFIG_BOOTSTRAP" || fail 'bootstrap source config wrote wrong values'

BOOTSTRAP_DIR="$TMP_DIR/bootstrap"
mkdir -p "$BOOTSTRAP_DIR/bin" "$BOOTSTRAP_DIR/lone" "$BOOTSTRAP_DIR/prefix/bin"
cp "$ROOT_DIR/install.sh" "$BOOTSTRAP_DIR/lone/install.sh"
cat >"$BOOTSTRAP_DIR/bin/curl" <<'SCRIPT'
#!/bin/sh
config=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -K)
            config="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done
[ -n "$config" ] || exit 2
grep -F 'url = "https://api.github.com/repos/example/private/tarball/dev"' "$config" >/dev/null || exit 3
grep -F 'header = "Authorization: Bearer github_pat_saved"' "$config" >/dev/null || exit 4
out="$(sed -n 's/^output = "\(.*\)"$/\1/p' "$config")"
[ -n "$out" ] || exit 5
printf 'archive\n' >"$out"
SCRIPT
cat >"$BOOTSTRAP_DIR/bin/tar" <<'SCRIPT'
#!/bin/sh
dest=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -C)
            dest="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done
[ -n "$dest" ] || exit 2
root="$dest/source"
mkdir -p "$root/bin" "$root/lib" "$root/tools" "$root/config"
cat >"$root/install.sh" <<'INNER'
#!/bin/sh
printf '%s\n' "$CODEX_TERMUX_BOOTSTRAPPED" >"$BOOTSTRAP_MARKER"
printf '%s\n' "$CODEX_TERMUX_BOOTSTRAP_REPO" >>"$BOOTSTRAP_MARKER"
printf '%s\n' "$CODEX_TERMUX_BOOTSTRAP_REF" >>"$BOOTSTRAP_MARKER"
printf '%s\n' "$CODEX_TERMUX_WRAPPER_SOURCE_DIR" >>"$BOOTSTRAP_MARKER"
printf '%s\n' "$*" >>"$BOOTSTRAP_MARKER"
INNER
printf 'runtime\n' >"$root/bin/install-runtime.sh"
printf 'lib\n' >"$root/lib/codex-termux.sh"
printf 'builder\n' >"$root/tools/build-runtime.py"
printf 'version\n' >"$root/config/wrapper-version.env"
chmod 755 "$root/install.sh"
SCRIPT
chmod 755 "$BOOTSTRAP_DIR/bin/curl" "$BOOTSTRAP_DIR/bin/tar"
cat >"$BOOTSTRAP_DIR/source.env" <<'ENV'
CODEX_TERMUX_WRAPPER_GIT_REPO=example/private
CODEX_TERMUX_WRAPPER_GIT_REF=dev
CODEX_TERMUX_WRAPPER_GIT_TOKEN=github_pat_saved
ENV
BOOTSTRAP_MARKER="$BOOTSTRAP_DIR/marker" \
PATH="$BOOTSTRAP_DIR/bin:$PATH" \
PREFIX="$BOOTSTRAP_DIR/prefix" \
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$BOOTSTRAP_DIR/source.env" \
    bash "$BOOTSTRAP_DIR/lone/install.sh" install-arg || fail 'single-file bootstrap failed'
grep -Fx '1' "$BOOTSTRAP_DIR/marker" >/dev/null || fail 'bootstrap marker did not record bootstrapped state'
grep -Fx 'example/private' "$BOOTSTRAP_DIR/marker" >/dev/null || fail 'bootstrap marker did not record repo'
grep -Fx 'dev' "$BOOTSTRAP_DIR/marker" >/dev/null || fail 'bootstrap marker did not record ref'
grep -F '/source' "$BOOTSTRAP_DIR/marker" >/dev/null || fail 'bootstrap marker did not record source dir'
grep -Fx 'install-arg' "$BOOTSTRAP_DIR/marker" >/dev/null || fail 'bootstrap did not preserve arguments'

printf 'wrapper-source-config: ok\n'
