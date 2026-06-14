#!/usr/bin/env bash
set -u

CODEX_NATIVE_HOME="${CODEX_NATIVE_HOME:-$HOME}"
CODEX_NATIVE_PREFIX="${PREFIX:-/data/data/com.termux/files/usr}"
CODEX_NATIVE_NATIVE_ROOT="${CODEX_NATIVE_NATIVE_ROOT:-$CODEX_NATIVE_HOME/.local/lib/codex/native}"
CODEX_NATIVE_RAW_DIR="${CODEX_NATIVE_RAW_DIR:-$CODEX_NATIVE_NATIVE_ROOT/raw}"
CODEX_NATIVE_RAW_VENDOR="${CODEX_NATIVE_RAW_VENDOR:-$CODEX_NATIVE_RAW_DIR/vendor/aarch64-unknown-linux-musl}"
CODEX_NATIVE_RUNTIME_DIR="${CODEX_NATIVE_RUNTIME_DIR:-$CODEX_NATIVE_NATIVE_ROOT/runtime}"
CODEX_NATIVE_RUNTIME="${CODEX_NATIVE_RUNTIME:-$CODEX_NATIVE_RUNTIME_DIR/codex}"
CODEX_NATIVE_MANAGED_SHELL="${CODEX_NATIVE_MANAGED_SHELL:-$CODEX_NATIVE_RUNTIME_DIR/managed.sh}"
CODEX_NATIVE_STATE_DIR="${CODEX_NATIVE_STATE_DIR:-$CODEX_NATIVE_HOME/.local/share/codex/native}"
CODEX_NATIVE_PROFILE_ROOT="${CODEX_NATIVE_PROFILE_ROOT:-$CODEX_NATIVE_HOME/.codex-profiles}"
CODEX_NATIVE_SHARED_PLUGINS_DIR="${CODEX_NATIVE_SHARED_PLUGINS_DIR:-$CODEX_NATIVE_HOME/.codex/plugins}"
CODEX_NATIVE_STATE_FILE="${CODEX_NATIVE_STATE_FILE:-$CODEX_NATIVE_STATE_DIR/state.json}"
CODEX_NATIVE_REGISTRY_FILE="${CODEX_NATIVE_REGISTRY_FILE:-$CODEX_NATIVE_STATE_DIR/registry.json}"
CODEX_NATIVE_STORE_DIR="${CODEX_NATIVE_STORE_DIR:-$CODEX_NATIVE_STATE_DIR/store}"
CODEX_NATIVE_BACKUP_DIR="${CODEX_NATIVE_BACKUP_DIR:-$CODEX_NATIVE_STATE_DIR/backups}"
CODEX_NATIVE_DOCTOR_DIR="${CODEX_NATIVE_DOCTOR_DIR:-$CODEX_NATIVE_STATE_DIR/doctor}"
CODEX_NATIVE_LOCK_FILE="${CODEX_NATIVE_LOCK_FILE:-$CODEX_NATIVE_STATE_DIR/native.lock}"
CODEX_NATIVE_LOCK_WAIT_SECONDS="${CODEX_NATIVE_LOCK_WAIT_SECONDS:-30}"
CODEX_NATIVE_RESOLV_CONF="${CODEX_NATIVE_RESOLV_CONF:-$CODEX_NATIVE_PREFIX/etc/resolv.conf}"
CODEX_NATIVE_CERT_FILE="${CODEX_NATIVE_CERT_FILE:-$CODEX_NATIVE_PREFIX/etc/tls/cert.pem}"
CODEX_NATIVE_CERT_DIR="${CODEX_NATIVE_CERT_DIR:-$CODEX_NATIVE_PREFIX/etc/tls/certs}"
CODEX_NATIVE_RESOLVER_FD="${CODEX_NATIVE_RESOLVER_FD:-33}"
CODEX_NATIVE_PACKAGE_SPEC_DEFAULT="${CODEX_NATIVE_PACKAGE_SPEC_DEFAULT:-@openai/codex@linux-arm64}"
CODEX_NATIVE_MANAGED_LAUNCHER_MARKER="${CODEX_NATIVE_MANAGED_LAUNCHER_MARKER:-codex native managed launcher}"
CODEX_NATIVE_PUBLIC_CODEX="${CODEX_NATIVE_PUBLIC_CODEX:-$CODEX_NATIVE_PREFIX/bin/codex}"
CODEX_NATIVE_RUNTIME_BUILDER="${CODEX_NATIVE_RUNTIME_BUILDER:-$CODEX_NATIVE_RUNTIME_DIR/build-runtime.py}"
CODEX_NATIVE_AUTO_UPDATE="${CODEX_NATIVE_AUTO_UPDATE:-1}"
CODEX_NATIVE_AUTO_UPDATE_MODE="${CODEX_NATIVE_AUTO_UPDATE_MODE:-prompt}"
CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS="${CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS:-21600}"
CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS="${CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS:-4}"
CODEX_NATIVE_AUTO_UPDATE_STAMP="${CODEX_NATIVE_AUTO_UPDATE_STAMP:-$CODEX_NATIVE_STATE_DIR/last-auto-update-check}"
CODEX_NATIVE_AUTO_UPDATE_PENDING="${CODEX_NATIVE_AUTO_UPDATE_PENDING:-$CODEX_NATIVE_STATE_DIR/pending-auto-update-version}"
CODEX_NATIVE_PROFILE_NETWORK_ACCESS="${CODEX_NATIVE_PROFILE_NETWORK_ACCESS:-1}"

codex_say() { printf 'codex: %s\n' "$*" >&2; }
codex_fail() { printf 'codex: ERROR: %s\n' "$*" >&2; return 1; }

codex_sha256() {
    sha256sum "$1" | awk '{print $1}'
}

codex_now() {
    date -Is
}

codex_file_has_marker() {
    local path="$1"
    [ -e "$path" ] || return 1
    grep -a -q "$CODEX_NATIVE_MANAGED_LAUNCHER_MARKER" "$path" 2>/dev/null
}

codex_with_lock() {
    local cmd="$1"
    shift
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    if command -v flock >/dev/null 2>&1; then
        (
            if ! flock -w "$CODEX_NATIVE_LOCK_WAIT_SECONDS" -x 9; then
                codex_fail "another mutation operation is in progress: $CODEX_NATIVE_LOCK_FILE"
                exit 75
            fi
            "$cmd" "$@"
        ) 9>"$CODEX_NATIVE_LOCK_FILE"
    else
        "$cmd" "$@"
    fi
}

codex_write_json_state() {
    local version="$1" raw_sha="$2" runtime_sha="$3" package_spec="$4" active_tuple_id="${5:-}"
    local wrapper_version wrapper_commit
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    python3 - "$CODEX_NATIVE_STATE_FILE" "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$active_tuple_id" "$wrapper_version" "$wrapper_commit" "$(codex_now)" <<'PY'
import json, os, sys
from pathlib import Path
path = Path(sys.argv[1])
data = {
    "schema": 2,
    "version": sys.argv[2],
    "raw_sha256": sys.argv[3],
    "runtime_sha256": sys.argv[4],
    "package_spec": sys.argv[5],
    "active_tuple_id": sys.argv[6],
    "wrapper_version": sys.argv[7],
    "wrapper_commit": sys.argv[8],
    "updated_at": sys.argv[9],
}
tmp = path.with_name("." + path.name + ".tmp")
path.parent.mkdir(parents=True, exist_ok=True)
tmp.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n")
os.chmod(tmp, 0o600)
os.replace(tmp, path)
PY
}

codex_read_state_field() {
    local field="$1"
    python3 - "$CODEX_NATIVE_STATE_FILE" "$field" <<'PY'
import json, sys
from pathlib import Path
path = Path(sys.argv[1])
field = sys.argv[2]
if not path.exists():
    print("")
else:
    print(json.loads(path.read_text()).get(field, ""))
PY
}

codex_record_registry() {
    local version="$1" raw_sha="$2" runtime_sha="$3" package_spec="$4" runtime_path="${5:-}"
    local wrapper_version wrapper_commit
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$runtime_path" "$wrapper_version" "$wrapper_commit" "$CODEX_NATIVE_STORE_DIR/runtime" "$(codex_now)" <<'PY'
import json, os, sys
from pathlib import Path
import re

def component(value: str, fallback: str = "unknown") -> str:
    value = value or fallback
    value = re.sub(r"[^A-Za-z0-9._+-]+", "_", value)
    return value or fallback

path = Path(sys.argv[1])
version = sys.argv[2]
raw_sha = sys.argv[3]
runtime_sha = sys.argv[4]
package_spec = sys.argv[5]
runtime_path = sys.argv[6]
wrapper_version = sys.argv[7]
wrapper_commit = sys.argv[8]
store_runtime_root = Path(sys.argv[9]).resolve()
updated_at = sys.argv[10]
raw_id = f"raw-{component(version)}-{component(raw_sha[:12])}"
wrapper_id = f"wrapper-{component(wrapper_version)}-{component(wrapper_commit[:12])}"
tuple_id = f"{raw_id}__{wrapper_id}"

def managed_runtime_path(value: str) -> bool:
    if not value:
        return False
    try:
        path_value = Path(value).resolve()
    except Exception:
        return False
    return (
        path_value.exists()
        and (path_value / "codex").exists()
        and (path_value == store_runtime_root or store_runtime_root in path_value.parents)
    )

entry = {
    "version": version,
    "raw_sha256": raw_sha,
    "runtime_sha256": runtime_sha,
    "package_spec": package_spec,
    "runtime_path": runtime_path,
    "updated_at": updated_at,
    "raw_id": raw_id,
    "wrapper_id": wrapper_id,
    "tuple_id": tuple_id,
}
if path.exists():
    try:
        data = json.loads(path.read_text())
    except Exception:
        data = {"schema": 2, "installs": []}
else:
    data = {"schema": 2, "installs": []}
data["schema"] = 2
data.setdefault("installs", [])
data.setdefault("raw", {})
data.setdefault("wrapper", {})
data.setdefault("runtime", {})
data["installs"] = [
    item for item in data.get("installs", [])
    if managed_runtime_path(item.get("runtime_path", ""))
]
data["runtime"] = {
    key: value for key, value in data.get("runtime", {}).items()
    if managed_runtime_path(value.get("path", ""))
}
data["raw"][raw_id] = {
    "version": version,
    "sha256": raw_sha,
    "package_spec": package_spec,
    "path": "raw/vendor/aarch64-unknown-linux-musl/bin/codex",
    "updated_at": updated_at,
}
data["wrapper"][wrapper_id] = {
    "version": wrapper_version,
    "commit": wrapper_commit,
    "repo": "local/codex",
    "updated_at": updated_at,
}
data["runtime"][tuple_id] = {
    "raw_id": raw_id,
    "wrapper_id": wrapper_id,
    "runtime_sha256": runtime_sha,
    "path": runtime_path,
    "smoke_tested_at": updated_at,
    "updated_at": updated_at,
}
data["active_tuple_id"] = tuple_id
data["installs"].insert(0, entry)
data["installs"] = data["installs"][:20]
tmp = path.with_name("." + path.name + ".tmp")
tmp.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n")
os.chmod(tmp, 0o600)
os.replace(tmp, path)
print(tuple_id)
PY
}

codex_store_id() {
    local version="$1" sha="$2"
    python3 - "$version" "$sha" <<'PY'
import re, sys
version = re.sub(r"[^A-Za-z0-9._+-]+", "_", sys.argv[1] or "unknown")
sha = (sys.argv[2] or "unknown")[:12]
print(f"{version}+{sha}")
PY
}

codex_store_runtime_payload() {
    local version="$1" runtime_sha="$2" store_id dst tmp
    store_id="$(codex_store_id "$version" "$runtime_sha")"
    dst="$CODEX_NATIVE_STORE_DIR/runtime/$store_id"
    tmp="$dst.tmp.$$"
    rm -rf "$tmp"
    mkdir -p "$tmp"
    cp -R "$CODEX_NATIVE_RUNTIME_DIR/codex" "$tmp/codex"
    cp -R "$CODEX_NATIVE_RUNTIME_DIR/codex-resources" "$tmp/codex-resources"
    cp -R "$CODEX_NATIVE_RUNTIME_DIR/codex-path" "$tmp/codex-path"
    cp -R "$CODEX_NATIVE_RUNTIME_DIR/codex-package.json" "$tmp/codex-package.json"
    rm -rf "$dst"
    mv "$tmp" "$dst"
    printf '%s\n' "$dst"
}

codex_promote_runtime_payload() {
    local src="$1" name target old
    [ -x "$src/codex" ] || return 1
    for name in codex codex-resources codex-path codex-package.json; do
        [ -e "$src/$name" ] || return 1
    done
    for name in codex codex-resources codex-path codex-package.json; do
        target="$CODEX_NATIVE_RUNTIME_DIR/$name"
        old="$CODEX_NATIVE_RUNTIME_DIR/.use-$name.old"
        rm -rf "$old"
        [ ! -e "$target" ] || mv "$target" "$old"
        cp -R "$src/$name" "$target"
        rm -rf "$old"
    done
    rm -f "$CODEX_NATIVE_RUNTIME_DIR/codex-resources/bwrap.real"
    chmod 755 "$CODEX_NATIVE_RUNTIME"
    chmod 755 \
        "$CODEX_NATIVE_RUNTIME_DIR/codex-resources/bwrap" \
        "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" \
        "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg" \
        "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg.real" 2>/dev/null || true
}

codex_package_spec() {
    local requested="${1:-}"
    if [ -z "$requested" ] || [ "$requested" = "latest" ] || [ "$requested" = "stable" ]; then
        printf '%s\n' "$CODEX_NATIVE_PACKAGE_SPEC_DEFAULT"
    elif [[ "$requested" == @openai/codex@* ]]; then
        printf '%s\n' "$requested"
    elif [[ "$requested" == *linux-arm64 ]]; then
        printf '@openai/codex@%s\n' "$requested"
    else
        printf '@openai/codex@%s-linux-arm64\n' "$requested"
    fi
}

codex_extract_pack_field() {
    local json_file="$1" field="$2"
    python3 - "$json_file" "$field" <<'PY'
import json, sys
data = json.loads(open(sys.argv[1]).read())
item = data[0] if isinstance(data, list) else data
print(item.get(sys.argv[2], ""))
PY
}

codex_fetch_package() {
    local requested="${1:-}" package_spec tmp pack_json tgz filename version
    package_spec="$(codex_package_spec "$requested")"
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/codex-pack.XXXXXX")" || return 1
    pack_json="$tmp/pack.json"
    codex_say "fetching $package_spec"
    if ! npm pack "$package_spec" --json --pack-destination "$tmp" >"$pack_json"; then
        rm -rf "$tmp"
        codex_fail "npm pack failed for $package_spec"
        return 1
    fi
    filename="$(codex_extract_pack_field "$pack_json" filename)"
    version="$(codex_extract_pack_field "$pack_json" version)"
    tgz="$tmp/$filename"
    if [ ! -f "$tgz" ]; then
        rm -rf "$tmp"
        codex_fail "npm pack did not create expected tarball"
        return 1
    fi
    mkdir -p "$tmp/package"
    if ! tar -xzf "$tgz" -C "$tmp/package" --strip-components=1; then
        rm -rf "$tmp"
        codex_fail "failed to extract $tgz"
        return 1
    fi
    printf '%s\t%s\t%s\t%s\n' "$tmp" "$tmp/package/vendor/aarch64-unknown-linux-musl" "$version" "$package_spec"
}

codex_install_raw_vendor() {
    local src_vendor="$1"
    rm -rf "$CODEX_NATIVE_RAW_DIR.new"
    mkdir -p "$CODEX_NATIVE_RAW_DIR.new/vendor"
    cp -R "$src_vendor" "$CODEX_NATIVE_RAW_DIR.new/vendor/aarch64-unknown-linux-musl"
    chmod 755 "$CODEX_NATIVE_RAW_DIR.new/vendor/aarch64-unknown-linux-musl/bin/codex"
    rm -rf "$CODEX_NATIVE_RAW_DIR.old"
    [ ! -e "$CODEX_NATIVE_RAW_DIR" ] || mv "$CODEX_NATIVE_RAW_DIR" "$CODEX_NATIVE_RAW_DIR.old"
    mv "$CODEX_NATIVE_RAW_DIR.new" "$CODEX_NATIVE_RAW_DIR"
    rm -rf "$CODEX_NATIVE_RAW_DIR.old"
}

codex_rebuild_runtime_unlocked() {
    local version="${1:-unknown}" package_spec="${2:-local}" report build_stdout raw_sha runtime_sha runtime_path tuple_id
    [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ] || return 1
    mkdir -p "$CODEX_NATIVE_STATE_DIR" "$CODEX_NATIVE_DOCTOR_DIR"
    report="$CODEX_NATIVE_DOCTOR_DIR/last-build-report.json"
    build_stdout="$CODEX_NATIVE_DOCTOR_DIR/last-build-report.stdout"
    if [ "${CODEX_NATIVE_BUILD_VERBOSE:-0}" = "1" ]; then
        "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_RAW_VENDOR" --runtime-dir "$CODEX_NATIVE_RUNTIME_DIR" --report-json "$report" || return 1
    else
        "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_RAW_VENDOR" --runtime-dir "$CODEX_NATIVE_RUNTIME_DIR" --report-json "$report" >"$build_stdout" || return 1
    fi
    raw_sha="$(codex_sha256 "$CODEX_NATIVE_RAW_VENDOR/bin/codex")"
    runtime_sha="$(codex_sha256 "$CODEX_NATIVE_RUNTIME")"
    runtime_path="$(codex_store_runtime_payload "$version" "$runtime_sha")"
    tuple_id="$(codex_record_registry "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$runtime_path")"
    codex_write_json_state "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$tuple_id"
}

codex_repair_runtime_from_raw_unlocked() {
    local version package_spec
    version="$(codex_read_state_field version)"
    package_spec="$(codex_read_state_field package_spec)"
    [ -n "$version" ] || version="unknown"
    [ -n "$package_spec" ] || package_spec="local"
    codex_rebuild_runtime_unlocked "$version" "$package_spec" || return $?
    codex_say "runtime repaired from cached raw package ($version)"
}

codex_repair_runtime_from_raw() {
    codex_with_lock codex_repair_runtime_from_raw_unlocked
}

codex_update_unlocked() {
    local requested="${1:-}" fetched tmp vendor version spec
    fetched="$(codex_fetch_package "$requested")" || return $?
    IFS=$'\t' read -r tmp vendor version spec <<EOF
$fetched
EOF
    codex_install_raw_vendor "$vendor" || {
        rm -rf "$tmp"
        return 1
    }
    codex_rebuild_runtime_unlocked "$version" "$spec" || {
        rm -rf "$tmp"
        return 1
    }
    rm -rf "$tmp"
    codex_say "installed Codex $version"
}

codex_update() {
    codex_with_lock codex_update_unlocked "${1:-}"
}

codex_refresh_support_from_source() {
    if [ -n "${CODEX_NATIVE_INSTALL_RUNTIME_SOURCE:-}" ] && [ -x "$CODEX_NATIVE_INSTALL_RUNTIME_SOURCE" ]; then
        bash "$CODEX_NATIVE_INSTALL_RUNTIME_SOURCE" support || codex_say "support refresh failed; keeping current wrapper"
    fi
}

codex_update_public() {
    codex_refresh_support_from_source
    codex_update "${1:-}"
    codex_version
}

codex_latest_linux_arm64_version() {
    if command -v timeout >/dev/null 2>&1; then
        timeout "$CODEX_NATIVE_AUTO_UPDATE_TIMEOUT_SECONDS" npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | tr -d '"'
    else
        npm view @openai/codex dist-tags.linux-arm64 --json 2>/dev/null | tr -d '"'
    fi
}

codex_auto_update_mode() {
    local mode="${CODEX_NATIVE_AUTO_UPDATE_MODE:-prompt}"
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
    [ "$CODEX_NATIVE_AUTO_UPDATE" = "0" ] && return 1
    [ "$(codex_auto_update_mode)" != "off" ] || return 1
    now="$(date +%s)"
    last="$(cat "$CODEX_NATIVE_AUTO_UPDATE_STAMP" 2>/dev/null || printf '0')"
    [ $((now - last)) -ge "$CODEX_NATIVE_AUTO_UPDATE_INTERVAL_SECONDS" ]
}

codex_mark_auto_update_checked() {
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    date +%s >"$CODEX_NATIVE_AUTO_UPDATE_STAMP"
}

codex_read_pending_auto_update() {
    cat "$CODEX_NATIVE_AUTO_UPDATE_PENDING" 2>/dev/null || true
}

codex_write_pending_auto_update() {
    local version="$1"
    mkdir -p "$CODEX_NATIVE_STATE_DIR"
    printf '%s\n' "$version" >"$CODEX_NATIVE_AUTO_UPDATE_PENDING"
}

codex_clear_pending_auto_update() {
    rm -f "$CODEX_NATIVE_AUTO_UPDATE_PENDING"
}

codex_prompt_update() {
    local current="$1" latest="$2" choice
    [ -t 0 ] && [ -t 2 ] || return 1
    printf 'codex: update available: %s -> %s\n' "$current" "$latest" >&2
    printf '  1) Run current patched runtime (default)\n' >&2
    printf '  0) Update now, patch, and run latest\n' >&2
    printf 'codex update [1/0]> ' >&2
    IFS= read -r -n 1 choice || return 1
    printf '\n' >&2
    case "$choice" in
        0|u|U|update|UPDATE|y|Y|yes|YES)
            return 0
            ;;
        *)
            codex_say "continuing with current patched runtime ($current)"
            return 1
            ;;
    esac
}

codex_install_auto_update() {
    local current="$1" latest="$2"
    codex_say "updating: $current -> $latest"
    codex_refresh_support_from_source
    if codex_update "$latest"; then
        codex_clear_pending_auto_update
    else
        codex_say "update failed; continuing with $current"
        return 1
    fi
}

codex_auto_update_if_needed() {
    local current latest mode pending
    codex_runtime_ok || return 0
    [ "$CODEX_NATIVE_AUTO_UPDATE" = "0" ] && return 0
    [ "$(codex_auto_update_mode)" != "off" ] || return 0
    current="$(codex_read_state_field version)"
    pending="$(codex_read_pending_auto_update)"
    if [ -n "$pending" ] && [ "$pending" = "$current" ]; then
        codex_clear_pending_auto_update
        pending=""
    fi
    if [ -n "$pending" ] && [ "$pending" != "$current" ]; then
        latest="$pending"
    else
        codex_auto_update_due || return 0
        codex_mark_auto_update_checked
        latest="$(codex_latest_linux_arm64_version || true)"
    fi
    [ -n "$latest" ] || return 0
    if [ "$latest" != "$current" ]; then
        codex_write_pending_auto_update "$latest"
        mode="$(codex_auto_update_mode)"
        if [ "$mode" = "force" ]; then
            codex_install_auto_update "$current" "$latest"
        elif codex_prompt_update "$current" "$latest"; then
            codex_install_auto_update "$current" "$latest"
        fi
    else
        codex_clear_pending_auto_update
    fi
}

codex_support_tools_match() {
    cmp -s "$CODEX_NATIVE_RUNTIME_DIR/bwrap-termux-compat.py" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" &&
    cmp -s "$CODEX_NATIVE_RUNTIME_DIR/rg-termux-shim.sh" "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg"
}

codex_runtime_ok() {
    [ -x "$CODEX_NATIVE_RUNTIME" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-resources/bwrap" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-path/bwrap" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg" ] &&
    [ -x "$CODEX_NATIVE_RUNTIME_DIR/codex-path/rg.real" ] &&
    codex_support_tools_match &&
    [ -r "$CODEX_NATIVE_STATE_FILE" ]
}

codex_ensure_runtime_ready() {
    codex_runtime_ok && return 0
    if [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ]; then
        codex_say "runtime drift detected; rebuilding from cached raw package"
        codex_repair_runtime_from_raw
        return $?
    fi
    codex_fail "runtime missing and no cached raw package is available; run codex setup"
    return 127
}

codex_detect_upstream_commands() {
    "$CODEX_NATIVE_RUNTIME" --help 2>/dev/null | python3 -c '
import re, sys
commands = set()
in_commands = False
for line in sys.stdin:
    if not in_commands:
        if re.match(r"^\s*Commands:\s*$", line):
            in_commands = True
        continue
    if re.match(r"^\s*(Arguments|Options):\s*$", line):
        break
    m = re.match(r"^\s{2,}([a-z0-9][a-z0-9-]*)\s{2,}", line, re.I)
    if not m:
        continue
    commands.add(m.group(1))
    am = re.search(r"\[aliases?: ([^\]]+)\]", line)
    if am:
        commands.update(a.strip() for a in am.group(1).split(",") if a.strip())
print("\n".join(sorted(commands)))
'
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
    codex_profile_enable_network_access "${CODEX_HOME:-$CODEX_NATIVE_HOME/.codex}"
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
    if [ -f "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env"
    fi
    printf '%s\n' "${CODEX_NATIVE_WRAPPER_VERSION:-unknown}"
}

codex_current_wrapper_commit() {
    if [ -f "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env" ]; then
        # shellcheck disable=SC1090
        . "$CODEX_NATIVE_RUNTIME_DIR/wrapper-version.env"
    fi
    printf '%s\n' "${CODEX_NATIVE_WRAPPER_COMMIT:-unknown}"
}

codex_version() {
    local upstream wrapper commit
    upstream="$("$CODEX_NATIVE_RUNTIME" --version 2>/dev/null || true)"
    wrapper="$(codex_current_wrapper_version)"
    commit="$(codex_current_wrapper_commit)"
    printf 'codex  : %s\n' "${upstream:-missing}"
    printf 'wrapper: %s (%s)\n' "$wrapper" "$commit"
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

codex_doctor_json() {
    local version raw_sha runtime_sha
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    python3 - "$CODEX_NATIVE_RUNTIME" "$CODEX_NATIVE_RUNTIME_DIR" "$CODEX_NATIVE_RAW_VENDOR" "$CODEX_NATIVE_RESOLV_CONF" "$CODEX_NATIVE_CERT_FILE" "$CODEX_NATIVE_STATE_FILE" "$CODEX_NATIVE_REGISTRY_FILE" "$version" "$raw_sha" "$runtime_sha" "$CODEX_NATIVE_PREFIX" <<'PY'
import filecmp, json, os, subprocess, sys
from pathlib import Path
runtime = Path(sys.argv[1])
runtime_dir = Path(sys.argv[2])
raw_vendor = Path(sys.argv[3])
resolv = Path(sys.argv[4])
cert = Path(sys.argv[5])
state = Path(sys.argv[6])
registry = Path(sys.argv[7])
version = sys.argv[8]
raw_sha = sys.argv[9]
runtime_sha = sys.argv[10]
prefix = Path(sys.argv[11])
path_bwrap = runtime_dir / "codex-path/bwrap"
bundled_bwrap = runtime_dir / "codex-resources/bwrap"
rg = runtime_dir / "codex-path/rg"
checks = {
    "runtime": runtime.exists() and os.access(runtime, os.X_OK),
    "raw": (raw_vendor / "bin/codex").exists(),
    "path_bwrap": path_bwrap.exists() and os.access(path_bwrap, os.X_OK),
    "bundled_bwrap": bundled_bwrap.exists() and os.access(bundled_bwrap, os.X_OK),
    "rg": rg.exists() and os.access(rg, os.X_OK),
    "rg_real": (runtime_dir / "codex-path/rg.real").exists(),
    "support_bwrap_match": filecmp.cmp(runtime_dir / "bwrap-termux-compat.py", path_bwrap, shallow=False) if (runtime_dir / "bwrap-termux-compat.py").exists() and path_bwrap.exists() else False,
    "support_rg_match": filecmp.cmp(runtime_dir / "rg-termux-shim.sh", rg, shallow=False) if (runtime_dir / "rg-termux-shim.sh").exists() and rg.exists() else False,
    "zsh": (runtime_dir / "codex-resources/zsh/bin/zsh").exists(),
    "resolv": resolv.exists() and os.access(resolv, os.R_OK),
    "cert": cert.exists() and os.access(cert, os.R_OK),
    "state": state.exists(),
    "registry": registry.exists(),
}
registry_data = {}
try:
    registry_data = json.loads(registry.read_text()) if registry.exists() else {}
except Exception:
    registry_data = {}
active_tuple_id = registry_data.get("active_tuple_id", "")
checks["registry_active_tuple"] = bool(active_tuple_id and active_tuple_id in registry_data.get("runtime", {}))
try:
    checks["bwrap_exec"] = subprocess.run(
        [str(path_bwrap), "--ro-bind", "/", "/", "--", str(prefix / "bin/true")],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    ).returncode == 0
except Exception:
    checks["bwrap_exec"] = False
try:
    checks["rg_exec"] = subprocess.run(
        [str(rg), "--version"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=10,
    ).returncode == 0
except Exception:
    checks["rg_exec"] = False
try:
    strings = subprocess.check_output(["strings", str(runtime)], text=True, errors="replace", timeout=10)
    checks["dns_patch"] = "/proc/self/fd/33" in strings and "/etc/resolv.conf" not in strings
except Exception:
    checks["dns_patch"] = False
print(json.dumps({
    "schema": 2,
    "overallStatus": "ok" if all(checks.values()) else "fail",
    "version": version,
    "raw_sha256": raw_sha,
    "runtime_sha256": runtime_sha,
    "paths": {
        "runtime": str(runtime),
        "raw_vendor": str(raw_vendor),
        "state": str(state),
        "registry": str(registry),
    },
    "activeTupleId": active_tuple_id,
    "termuxDelta": {
        "browserLogin": "termux-open-url when available",
        "bwrap": "runtime-private quiet no-namespace compatibility launcher",
        "codexSelfExe": "managed runtime",
        "ldLibraryPath": "sanitized before runtime execution",
        "runtimePatch": "official linux-arm64 raw package rebuilt into Termux-managed runtime",
    },
    "checks": checks,
}, ensure_ascii=True, sort_keys=True))
PY
}

codex_doctor() {
    if [ "${1:-}" = "--upstream" ]; then
        shift
        codex_open_fd33_and_exec doctor "$@"
    elif [ "${1:-}" = "--json" ]; then
        codex_doctor_json
    else
        codex_doctor_json | python3 -m json.tool
    fi
}

codex_profile_validate_name() {
    local profile="${1:-}"
    case "$profile" in
        ""|default)
            return 0
            ;;
        native|-*|.*|*/*|*..*|*[[:space:]]*)
            return 1
            ;;
        *)
            return 0
            ;;
    esac
}

codex_profile_dir() {
    local profile="${1:-default}"
    if [ -z "$profile" ] || [ "$profile" = "default" ]; then
        printf '%s\n' "$CODEX_NATIVE_HOME/.codex"
    else
        printf '%s/%s\n' "$CODEX_NATIVE_PROFILE_ROOT" "$profile"
    fi
}

codex_profile_share_plugins() {
    local profile_dir="$1" shared_plugins_dir="$CODEX_NATIVE_SHARED_PLUGINS_DIR" plugins_dir
    plugins_dir="$profile_dir/plugins"
    mkdir -p "$shared_plugins_dir"
    if [ -e "$plugins_dir" ] || [ -L "$plugins_dir" ]; then
        return 0
    fi
    ln -s "$shared_plugins_dir" "$plugins_dir"
}

codex_profile_network_access_enabled() {
    case "${CODEX_NATIVE_PROFILE_NETWORK_ACCESS:-1}" in
        0|off|false|no|none)
            return 1
            ;;
        *)
            return 0
            ;;
    esac
}

codex_profile_enable_network_access() {
    local profile_dir="$1" config_file
    codex_profile_network_access_enabled || return 0
    config_file="$profile_dir/config.toml"
    mkdir -p "$profile_dir"
    python3 - "$config_file" <<'PY'
import os
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
text = path.read_text() if path.exists() else ""
lines = text.splitlines()
section_re = re.compile(r"^\s*\[([^\]]+)\]\s*(?:#.*)?$")
network_re = re.compile(r"^\s*network_access\s*=")

out = []
in_section = False
found_section = False
found_key = False
changed = False

def add_missing_key_if_needed() -> None:
    global found_key, changed
    if in_section and not found_key:
        out.append("network_access = true")
        found_key = True
        changed = True

for line in lines:
    match = section_re.match(line)
    if match:
        add_missing_key_if_needed()
        in_section = match.group(1).strip() == "sandbox_workspace_write"
        if in_section:
            found_section = True
            found_key = False
        out.append(line)
        continue
    if in_section and network_re.match(line):
        if line.strip() != "network_access = true":
            out.append("network_access = true")
            changed = True
        else:
            out.append(line)
        found_key = True
        continue
    out.append(line)

add_missing_key_if_needed()

if not found_section:
    if out and out[-1] != "":
        out.append("")
    out.extend(["[sandbox_workspace_write]", "network_access = true"])
    changed = True

new_text = "\n".join(out) + "\n"
if not path.exists() or changed or text != new_text:
    tmp = path.with_name("." + path.name + ".tmp")
    tmp.write_text(new_text)
    if path.exists():
        os.chmod(tmp, path.stat().st_mode & 0o777)
    else:
        os.chmod(tmp, 0o600)
    os.replace(tmp, path)
PY
}

codex_list_profiles() {
    local root="$CODEX_NATIVE_PROFILE_ROOT"
    [ -d "$root" ] || return 0
    find "$root" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null \
        | grep -Ev '^(default|native)$' \
        | grep -Ev '^[.]' \
        | LC_ALL=C sort -f
}

codex_prompt_choice() {
    local prompt="${1:-choose> }" max_items="${2:-9}" reply rest old_tty status
    printf '%s' "$prompt" >&2
    if [ -t 0 ]; then
        old_tty="$(stty -g 2>/dev/null || true)"
        [ -z "$old_tty" ] || stty -echo -icanon min 1 time 0 2>/dev/null || true
        IFS= read -r -N 1 reply
        status=$?
        [ -z "$old_tty" ] || stty "$old_tty" 2>/dev/null || true
        if [ "$status" -ne 0 ]; then
            printf '\n' >&2
            return 1
        fi
    elif ! IFS= read -r -N 1 reply; then
        printf '\n' >&2
        return 1
    fi
    case "$reply" in
        $'\e')
            printf '\n' >&2
            return 130
            ;;
        $'\n'|$'\r'|'')
            printf '\n' >&2
            return 0
            ;;
        [0-9])
            if [ "$max_items" -le 9 ]; then
                printf '%s\n' "$reply" >&2
                printf '%s\n' "$reply"
                return 0
            fi
            ;;
        *)
            ;;
    esac
    rest=""
    printf '%s' "$reply" >&2
    IFS= read -r rest || true
    printf '%s%s\n' "$reply" "$rest"
    return 0
}

codex_profile_exec() {
    local profile_dir="$1"
    shift || true
    if [ ! -d "$profile_dir" ]; then
        codex_fail "profile directory not found: $profile_dir"
        return 2
    fi
    codex_profile_share_plugins "$profile_dir"
    codex_profile_enable_network_access "$profile_dir"
    codex_ensure_runtime_ready || return $?
    codex_auto_update_if_needed
    codex_prepare_runtime_env
    CODEX_HOME="$profile_dir" exec "$CODEX_NATIVE_RUNTIME" "$@"
}

codex_profile_select() {
    local profiles=() profile choice idx profile_dir display_limit=0 truncated=0
    mapfile -t profiles < <(codex_list_profiles)
    if [ -t 0 ]; then
        display_limit=9
    fi

    printf 'Choose profile\n' >&2
    printf '   0. default\n' >&2
    idx=1
    for profile in "${profiles[@]}"; do
        if [ "$display_limit" -gt 0 ] && [ "$idx" -gt "$display_limit" ]; then
            truncated=1
            break
        fi
        printf '  %2d. %s\n' "$idx" "$profile" >&2
        idx=$((idx + 1))
    done
    if [ "$truncated" -eq 1 ]; then
        printf '  (More options: codex profile NAME)\n' >&2
    fi
    printf '\n' >&2

    if [ ! -t 0 ]; then
        return 0
    fi

    choice="$(codex_prompt_choice 'choose profile > ' "$(( ${#profiles[@]} < 9 ? ${#profiles[@]} : 9 ))")" || return $?
    [ -n "$choice" ] || return 130

    if [ "$choice" = "0" ] || [ "$choice" = "default" ]; then
        profile="default"
    elif [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le "${#profiles[@]}" ]; then
        profile="${profiles[$((choice - 1))]}"
    else
        profile="$choice"
    fi

    codex_profile_validate_name "$profile" || {
        codex_fail "invalid profile name: $profile"
        return 2
    }
    profile_dir="$(codex_profile_dir "$profile")"
    codex_profile_exec "$profile_dir"
}

codex_profile_run() {
    local profile="${1:-}"
    if [ -z "$profile" ]; then
        codex_profile_select
        return $?
    fi
    codex_profile_validate_name "$profile" || {
        codex_fail "invalid profile name: $profile"
        return 2
    }
    local profile_dir
    profile_dir="$(codex_profile_dir "$profile")"
    shift || true
    codex_profile_exec "$profile_dir" "$@"
}

codex_restore_backup() {
    local public="$1" base latest
    base="$(basename "$public")"
    latest="$(ls -t "$CODEX_NATIVE_BACKUP_DIR"/"$base".*.bak 2>/dev/null | sed -n '1p' || true)"
    if [ -n "$latest" ]; then
        cp -Pp "$latest" "$public"
        codex_say "restored $public from $latest"
    fi
}

codex_remove() {
    if codex_file_has_marker "$CODEX_NATIVE_PUBLIC_CODEX"; then
        rm -f "$CODEX_NATIVE_PUBLIC_CODEX"
        codex_restore_backup "$CODEX_NATIVE_PUBLIC_CODEX"
    fi
    rm -rf "$CODEX_NATIVE_NATIVE_ROOT"
    codex_say "removed managed runtime; state kept at $CODEX_NATIVE_STATE_DIR for backups"
}

codex_use() {
    local choice="${1:-}" runtime_path version raw_sha runtime_sha package_spec
    if [ "$choice" = "--list" ]; then
        codex_use_list list
        return $?
    fi
    if [ -n "$choice" ]; then
        codex_use_select "$choice"
        return $?
    fi
    codex_use_list menu
    if [ ! -t 0 ]; then
        return 0
    fi
    choice="$(codex_prompt_choice 'choose runtime > ' "${CODEX_USE_MENU_COUNT:-0}")" || return $?
    if [ -z "$choice" ]; then
        printf 'codex use: cancelled.\n' >&2
        return 1
    fi
    codex_use_select "$choice"
}

codex_use_list() {
    local mode="${1:-list}" latest interactive_limit=0
    latest="$(codex_latest_linux_arm64_version || true)"
    CODEX_USE_LAST_LATEST="$latest"
    if [ "$mode" = "menu" ] && [ -t 0 ]; then
        interactive_limit=9
    fi
    if [ "$mode" = "list" ]; then
        CODEX_USE_MENU_COUNT=0
        codex_use_render "$latest" "$interactive_limit" "$mode"
    else
        CODEX_USE_MENU_COUNT="$(codex_use_render "$latest" "$interactive_limit" "$mode")"
    fi
}

codex_use_render() {
    local latest="$1" interactive_limit="$2" mode="$3"
    python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$latest" "$CODEX_NATIVE_STORE_DIR/runtime" "$interactive_limit" "$mode" <<'PY'
import json, os, sys
from pathlib import Path
path = Path(sys.argv[1])
latest = sys.argv[2]
store_runtime_root = Path(sys.argv[3]).resolve()
interactive_limit = int(sys.argv[4])
mode = sys.argv[5]
data = json.loads(path.read_text()) if path.exists() else {"installs": []}
active_tuple_id = data.get("active_tuple_id", "")

def managed_runtime_path(value: str) -> bool:
    if not value:
        return False
    try:
        runtime_path = Path(value).resolve()
    except Exception:
        return False
    return (
        (runtime_path / "codex").exists()
        and (runtime_path == store_runtime_root or store_runtime_root in runtime_path.parents)
    )

seen = set()
rows = []
for entry in data.get("installs", []):
    runtime_path = entry.get("runtime_path", "")
    key = entry.get("tuple_id", "") or runtime_path
    if key in seen or not managed_runtime_path(runtime_path):
        continue
    seen.add(key)
    rows.append(entry)

count = 0
truncated = False

if mode == "list":
    for index, entry in enumerate(rows, 1):
        print("\t".join([
            str(index),
            "cached",
            entry.get("version", "unknown"),
            entry.get("runtime_sha256", "")[:12],
            entry.get("package_spec", ""),
            entry.get("runtime_path", ""),
        ]))
    cached_versions = {entry.get("version", "") for entry in rows}
    if latest and latest not in cached_versions:
        print("\t".join([
            str(len(rows) + 1),
            "remote",
            latest,
            "",
            f"@openai/codex@{latest}",
            "npm:linux-arm64",
        ]))
    if not rows and not latest:
        print("no cached runtimes", file=sys.stderr)
        raise SystemExit(1)
    raise SystemExit(0)

print("Choose runtime", file=sys.stderr)
for entry in rows:
    if interactive_limit and count >= interactive_limit:
        truncated = True
        continue
    count += 1
    version = entry.get("version", "unknown")
    tuple_id = entry.get("tuple_id", "")
    wrapper_id = entry.get("wrapper_id", "")
    wrapper_short = wrapper_id.split("wrapper-", 1)[-1] if wrapper_id else "wrapper-unknown"
    badges = ["active" if tuple_id == active_tuple_id else None, "cached"]
    badge_text = ", ".join(item for item in badges if item)
    print(f"  {count:2d}. {version}  [{wrapper_short}]  {badge_text}", file=sys.stderr)

cached_versions = {entry.get("version", "") for entry in rows}
if latest and latest not in cached_versions:
    if not interactive_limit or count < interactive_limit:
        count += 1
        print(f"  {count:2d}. {latest}  [latest wrapper]  remote", file=sys.stderr)
    else:
        truncated = True
if not rows and not latest:
    print("no cached runtimes", file=sys.stderr)
    raise SystemExit(1)
if truncated:
    print("  (More options: codex use <version>)", file=sys.stderr)
print(file=sys.stderr)
print(count)
PY
}

codex_use_select() {
    local choice="$1" selected tuple_id
    local latest="${CODEX_USE_LAST_LATEST:-}"
    if [ -z "$latest" ]; then
        latest="$(codex_latest_linux_arm64_version || true)"
    fi
    selected="$(python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$choice" "$latest" "$CODEX_NATIVE_STORE_DIR/runtime" <<'PY'
import json, sys
from pathlib import Path
path = Path(sys.argv[1])
choice = sys.argv[2]
latest = sys.argv[3]
store_runtime_root = Path(sys.argv[4]).resolve()
data = json.loads(path.read_text()) if path.exists() else {"installs": []}

def managed_runtime_path(value: str) -> bool:
    if not value:
        return False
    try:
        runtime_path = Path(value).resolve()
    except Exception:
        return False
    return (
        (runtime_path / "codex").exists()
        and (runtime_path == store_runtime_root or store_runtime_root in runtime_path.parents)
    )

rows = []
seen = set()
for entry in data.get("installs", []):
    runtime_path = entry.get("runtime_path", "")
    key = entry.get("tuple_id", "") or runtime_path
    if key in seen or not managed_runtime_path(runtime_path):
        continue
    seen.add(key)
    rows.append(("cached", entry))
match = None
if choice.isdigit() and 1 <= int(choice) <= len(rows):
    match = rows[int(choice) - 1]
else:
    for kind, entry in rows:
        if choice in (entry.get("version", ""), entry.get("runtime_sha256", "")[:12]):
            match = (kind, entry)
            break
if not match:
    cached_versions = {entry.get("version", "") for _, entry in rows}
    if latest and latest not in cached_versions:
        remote_entry = {
            "version": latest,
            "runtime_sha256": "",
            "raw_sha256": "",
            "package_spec": f"@openai/codex@{latest}",
            "runtime_path": "npm:linux-arm64",
        }
        rows.append(("remote", remote_entry))
        if choice.isdigit() and int(choice) == len(rows):
            match = rows[-1]
        elif choice == latest:
            match = rows[-1]
if not match:
    raise SystemExit(1)
kind, entry = match
print("\t".join([
    kind,
    entry.get("runtime_path", ""),
    entry.get("version", "unknown"),
    entry.get("raw_sha256", ""),
    entry.get("runtime_sha256", ""),
    entry.get("package_spec", ""),
]))
PY
)" || {
        codex_fail "unknown cached runtime selection: $choice"
        return 1
    }
    local kind
    IFS=$'\t' read -r kind runtime_path version raw_sha runtime_sha package_spec <<EOF
$selected
EOF
    if [ "$kind" = "remote" ]; then
        codex_update "$version" || return $?
    else
        codex_promote_runtime_payload "$runtime_path" || return 1
        tuple_id="$(codex_record_registry "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$runtime_path")"
        codex_write_json_state "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$tuple_id"
        codex_say "using Codex $version"
    fi
    codex_version
}

codex_bootstrap_store() {
    local version raw_sha runtime_sha package_spec runtime_path tuple_id
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    package_spec="$(codex_read_state_field package_spec)"
    [ -n "$version" ] && [ -n "$runtime_sha" ] && [ -x "$CODEX_NATIVE_RUNTIME" ] || return 0
    runtime_path="$(codex_store_runtime_payload "$version" "$runtime_sha")"
    tuple_id="$(codex_record_registry "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$runtime_path")"
    codex_write_json_state "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$tuple_id"
}

codex_setup_public() {
    if [ -n "${CODEX_NATIVE_INSTALL_RUNTIME_SOURCE:-}" ] && [ -x "$CODEX_NATIVE_INSTALL_RUNTIME_SOURCE" ]; then
        exec bash "$CODEX_NATIVE_INSTALL_RUNTIME_SOURCE" setup "$@"
    fi
    codex_update "${1:-}"
}

codex_main() {
    case "${1:-}" in
        setup)
            shift
            codex_setup_public "$@"
            ;;
        update)
            shift
            codex_update_public "${1:-}"
            ;;
        doctor)
            shift
            codex_doctor "$@"
            ;;
        version)
            shift
            codex_version
            ;;
        help|--help|-h)
            shift || true
            codex_help "$@"
            ;;
        use)
            shift
            codex_use "$@"
            ;;
        profile)
            shift
            codex_profile_run "$@"
            ;;
        remove)
            shift
            codex_remove
            ;;
        *)
            codex_ensure_runtime_ready || return $?
            codex_auto_update_if_needed
            codex_open_fd33_and_exec "$@"
            ;;
    esac
}
