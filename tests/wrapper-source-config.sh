#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
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
    CODEX_TERMUX_WRAPPER_TOKEN="github_pat_test" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private dev >/dev/null \
    || fail 'install.sh source failed'

[ -f "$CONFIG" ] || fail 'source config was not created'
[ "$(file_mode "$CONFIG")" = "600" ] || fail "source config mode is not 600: $(file_mode "$CONFIG")"

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_TOKEN" = "github_pat_test" ]
' _ "$CONFIG" || fail 'source config did not restore saved values'

CODEX_TERMUX_WRAPPER_REPO="override/repo" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REPO" = "override/repo" ]
' _ "$CONFIG" || fail 'explicit environment did not override saved repo'

CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_TOKEN" = "github_pat_test" ]
' _ "$ROOT_DIR/bin/install-runtime.sh" || fail 'install-runtime did not auto-load source config'

CONFIG_DEFAULT_REF="$TMP_DIR/wrapper-source-default-ref.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_DEFAULT_REF" \
    CODEX_TERMUX_WRAPPER_TOKEN="github_pat_test" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private "" >/dev/null \
    || fail 'install.sh source with empty ref failed'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REF" = "main" ]
' _ "$CONFIG_DEFAULT_REF" || fail 'empty source ref did not default to main'

CONFIG_REUSE="$TMP_DIR/wrapper-source-reuse.env"
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_REUSE" \
    CODEX_TERMUX_WRAPPER_TOKEN="saved_pat_test" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private main >/dev/null \
    || fail 'initial source config for token reuse failed'

CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_REUSE" \
    bash "$ROOT_DIR/install.sh" source --save-only example/private dev </dev/null >/dev/null \
    || fail 'source config did not reuse saved token'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_TOKEN" = "saved_pat_test" ] &&
    [ "$CODEX_TERMUX_WRAPPER_REF" = "dev" ]
' _ "$CONFIG_REUSE" || fail 'source config token reuse wrote wrong values'

CONFIG_LEGACY="$TMP_DIR/wrapper-source-legacy.env"
cat >"$CONFIG_LEGACY" <<'ENV'
CODEX_TERMUX_WRAPPER_GIT_REPO=legacy/private
CODEX_TERMUX_WRAPPER_GIT_REF=legacy-branch
CODEX_TERMUX_WRAPPER_GIT_TOKEN=legacy_pat_test
ENV
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_LEGACY" \
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REPO" = "legacy/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_REF" = "legacy-branch" ] &&
    [ "$CODEX_TERMUX_WRAPPER_TOKEN" = "legacy_pat_test" ]
' _ "$ROOT_DIR/bin/install-runtime.sh" || fail 'legacy source config was not normalized by install-runtime'

CONFIG_LEGACY_PERSIST="$TMP_DIR/wrapper-source-legacy-persist.env"
cat >"$CONFIG_LEGACY_PERSIST" <<'ENV'
CODEX_TERMUX_WRAPPER_GIT_REPO=legacy/persist
CODEX_TERMUX_WRAPPER_GIT_REF=legacy-main
CODEX_TERMUX_WRAPPER_GIT_TOKEN=legacy_persist_pat
ENV
CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$CONFIG_LEGACY_PERSIST" \
bash -lc '
    . "$1"
    migrate_wrapper_source_config_if_needed
' _ "$ROOT_DIR/install.sh" || fail 'legacy source config persistence migration failed'
bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REPO" = "legacy/persist" ] &&
    [ "$CODEX_TERMUX_WRAPPER_REF" = "legacy-main" ] &&
    [ "$CODEX_TERMUX_WRAPPER_TOKEN" = "legacy_persist_pat" ]
' _ "$CONFIG_LEGACY_PERSIST" || fail 'legacy source config was not rewritten with canonical keys'

bash -lc '
    . "$1"
    TMP_GH="$2"
    mkdir -p "$TMP_GH"
    cat >"$TMP_GH/gh" <<'"'"'SCRIPT'"'"'
#!/bin/sh
[ "$1" = "auth" ] && [ "$2" = "token" ] || exit 3
printf "%s\n" gh_token_test
SCRIPT
    chmod 755 "$TMP_GH/gh"
    PATH="$TMP_GH:$PATH"
    GITHUB_TOKEN=
    CODEX_TERMUX_WRAPPER_TOKEN=
    [ "$(bootstrap_auth_token)" = "gh_token_test" ]
' _ "$ROOT_DIR/install.sh" "$TMP_DIR" || fail 'gh auth token fallback did not resolve bootstrap token'

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

LOCAL_DIR="$TMP_DIR/local-checkout"
mkdir -p "$LOCAL_DIR/bin" "$LOCAL_DIR/lib/codex-termux" "$LOCAL_DIR/tools" "$LOCAL_DIR/config"
cp "$ROOT_DIR/install.sh" "$LOCAL_DIR/install.sh"
cat >"$LOCAL_DIR/bin/install-local.sh" <<'SCRIPT'
#!/bin/sh
printf '%s\n' "$*" >"$LOCAL_MARKER"
SCRIPT
printf 'runtime\n' >"$LOCAL_DIR/bin/install-runtime.sh"
printf 'lib\n' >"$LOCAL_DIR/lib/codex-termux.sh"
for domain in dispatch state profile session runtime notify doctor; do printf '%s\n' "$domain" >"$LOCAL_DIR/lib/codex-termux/$domain.sh"; done
printf '{}\n' >"$LOCAL_DIR/codex-wrapper.manifest.json"
printf 'builder\n' >"$LOCAL_DIR/tools/build-runtime.py"
printf 'version\n' >"$LOCAL_DIR/config/wrapper-version.env"
chmod 755 "$LOCAL_DIR/install.sh" "$LOCAL_DIR/bin/install-local.sh"
LOCAL_MARKER="$TMP_DIR/local-marker" bash "$LOCAL_DIR/install.sh" local-arg >/dev/null \
    || fail 'checkout install.sh did not delegate to install-local'
grep -Fx 'local-arg' "$TMP_DIR/local-marker" >/dev/null \
    || fail 'checkout install.sh delegation did not preserve arguments'

bash -lc '
    . "$1"
    PREFIX="$2/prefix"
    mkdir -p "$PREFIX/bin"
    printf "#!/bin/sh\nexit 0\n" >"$PREFIX/bin/pkg"
    printf "#!/bin/sh\nexit 0\n" >"$PREFIX/bin/codex"
    chmod 755 "$PREFIX/bin/pkg" "$PREFIX/bin/codex"
    CODEX_TERMUX_WRAPPER_SOURCE_CONFIG="$2/wrapper-source.env"
    printf "%s\n" \
        "CODEX_TERMUX_WRAPPER_REPO=example/private" \
        >"$CODEX_TERMUX_WRAPPER_SOURCE_CONFIG"
    install_dependencies() { return 0; }
    bash() {
        case "$1 $2" in
            "$ROOT_DIR/bin/install-runtime.sh install")
                [ "${CODEX_TERMUX_WRAPPER_SOURCE_DIR:-}" = "$ROOT_DIR" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_REPO:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_REF:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_GIT_URL:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_GIT_REPO:-}" ] &&
                [ -z "${CODEX_TERMUX_WRAPPER_GIT_REF:-}" ] &&
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
    CODEX_TERMUX_WRAPPER_TOKEN=github_pat_bootstrap
    save_current_source_config_if_missing
' _ "$ROOT_DIR/install.sh" "$CONFIG_BOOTSTRAP" || fail 'bootstrap source config was not saved'

bash -lc '
    . "$1"
    [ "$CODEX_TERMUX_WRAPPER_REPO" = "example/private" ] &&
    [ "$CODEX_TERMUX_WRAPPER_REF" = "dev" ] &&
    [ "$CODEX_TERMUX_WRAPPER_TOKEN" = "github_pat_bootstrap" ]
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
python3 - "$out" <<'PYTHON'
import io
import sys
import tarfile

out = sys.argv[1]
install_local = "\n".join([
    "#!/bin/sh",
    "printf '%s\\n' \"$CODEX_TERMUX_BOOTSTRAPPED\" >\"$BOOTSTRAP_MARKER\"",
    "printf '%s\\n' \"$CODEX_TERMUX_BOOTSTRAP_REPO\" >>\"$BOOTSTRAP_MARKER\"",
    "printf '%s\\n' \"$CODEX_TERMUX_BOOTSTRAP_REF\" >>\"$BOOTSTRAP_MARKER\"",
    "printf '%s\\n' \"$CODEX_TERMUX_WRAPPER_SOURCE_DIR\" >>\"$BOOTSTRAP_MARKER\"",
    "printf '%s\\n' \"$*\" >>\"$BOOTSTRAP_MARKER\"",
    "",
])
entries = {
    "source/install.sh": "#!/bin/sh\nexit 99\n",
    "source/bin/install-local.sh": install_local,
    "source/bin/install-runtime.sh": "runtime\n",
    "source/lib/codex-termux.sh": "lib\n",
    "source/lib/codex-termux/dispatch.sh": "dispatch\n",
    "source/lib/codex-termux/state.sh": "state\n",
    "source/lib/codex-termux/profile.sh": "profile\n",
    "source/lib/codex-termux/session.sh": "session\n",
    "source/lib/codex-termux/runtime.sh": "runtime\n",
    "source/lib/codex-termux/notify.sh": "notify\n",
    "source/lib/codex-termux/doctor.sh": "doctor\n",
    "source/codex-wrapper.manifest.json": "{}\n",
    "source/tools/build-runtime.py": "builder\n",
    "source/config/wrapper-version.env": "version\n",
}
with tarfile.open(out, "w:gz") as tf:
    for name in ["source", "source/bin", "source/lib", "source/lib/codex-termux", "source/tools", "source/config"]:
        info = tarfile.TarInfo(name)
        info.type = tarfile.DIRTYPE
        info.mode = 0o755
        tf.addfile(info)
    for name, text in entries.items():
        data = text.encode("utf-8")
        info = tarfile.TarInfo(name)
        info.size = len(data)
        info.mode = 0o755 if name.endswith(("install.sh", "install-local.sh")) else 0o644
        tf.addfile(info, io.BytesIO(data))
PYTHON
SCRIPT
chmod 755 "$BOOTSTRAP_DIR/bin/curl"
cat >"$BOOTSTRAP_DIR/source.env" <<'ENV'
CODEX_TERMUX_WRAPPER_REPO=example/private
CODEX_TERMUX_WRAPPER_REF=dev
CODEX_TERMUX_WRAPPER_TOKEN=github_pat_saved
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
