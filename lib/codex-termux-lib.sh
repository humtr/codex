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
CODEX_NATIVE_RUNTIME_RETENTION="${CODEX_NATIVE_RUNTIME_RETENTION:-3}"
CODEX_NATIVE_PATCH_POLICY="${CODEX_NATIVE_PATCH_POLICY:-dns-fd33-only-v1}"

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
    "schema": 3,
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
        data = {"schema": 3, "installs": []}
else:
    data = {"schema": 3, "installs": []}
data["schema"] = 3
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

codex_validate_runtime_retention() {
    case "$CODEX_NATIVE_RUNTIME_RETENTION" in
        ''|*[!0-9]*|0)
            codex_fail "CODEX_NATIVE_RUNTIME_RETENTION must be an integer greater than zero"
            return 2
            ;;
    esac
}

codex_prune_runtime_store() {
    codex_validate_runtime_retention || return $?
    python3 - "$CODEX_NATIVE_STORE_DIR/runtime" "$CODEX_NATIVE_REGISTRY_FILE" \
        "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_PATCH_POLICY" \
        "$CODEX_NATIVE_RUNTIME_RETENTION" <<'PY'
import hashlib, json, shutil, sys
from pathlib import Path

store, registry_path, builder = map(Path, sys.argv[1:4])
policy = sys.argv[4]
retention = int(sys.argv[5])

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

builder_sha = sha256(builder)
try:
    data = json.loads(registry_path.read_text()) if registry_path.exists() else {}
except Exception:
    data = {}
active_tuple = data.get("active_tuple_id", "")
active_path = ""
if active_tuple:
    active_path = data.get("runtime", {}).get(active_tuple, {}).get("path", "")
try:
    active_path = str(Path(active_path).resolve()) if active_path else ""
except Exception:
    active_path = ""

compatible = []
if store.exists():
    for path in store.iterdir():
        if not path.is_dir():
            continue
        try:
            manifest = json.loads((path / "runtime-build.json").read_text())
            ok = (
                manifest.get("patch_policy") == policy
                and manifest.get("builder_sha256") == builder_sha
                and manifest.get("runtime_sha256") == sha256(path / "codex")
            )
        except Exception:
            ok = False
        if ok:
            compatible.append(path)
        else:
            shutil.rmtree(path)

compatible.sort(key=lambda item: item.stat().st_mtime, reverse=True)
keep = []
active = next((path for path in compatible if str(path.resolve()) == active_path), None)
if active:
    keep.append(active)
for path in compatible:
    if path not in keep and len(keep) < retention:
        keep.append(path)
for path in compatible:
    if path not in keep:
        shutil.rmtree(path)

kept = {str(path.resolve()) for path in keep}
runtime = {}
for key, value in data.get("runtime", {}).items():
    try:
        resolved = str(Path(value.get("path", "")).resolve())
    except Exception:
        continue
    if resolved in kept:
        runtime[key] = value
data["runtime"] = runtime

installs = []
seen = set()
for entry in data.get("installs", []):
    try:
        resolved = str(Path(entry.get("runtime_path", "")).resolve())
    except Exception:
        continue
    key = entry.get("tuple_id", "") or resolved
    if resolved in kept and key not in seen:
        installs.append(entry)
        seen.add(key)
data["installs"] = installs
referenced_raw = {entry.get("raw_id", "") for entry in installs}
referenced_wrapper = {entry.get("wrapper_id", "") for entry in installs}
data["raw"] = {key: value for key, value in data.get("raw", {}).items() if key in referenced_raw}
data["wrapper"] = {key: value for key, value in data.get("wrapper", {}).items() if key in referenced_wrapper}
if active_tuple not in runtime:
    data["active_tuple_id"] = ""

if registry_path.exists():
    tmp = registry_path.with_name("." + registry_path.name + ".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=True, sort_keys=True) + "\n")
    tmp.chmod(0o600)
    tmp.replace(registry_path)
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
    cp -R "$CODEX_NATIVE_RUNTIME_DIR/runtime-build.json" "$tmp/runtime-build.json"
    rm -rf "$dst"
    mv "$tmp" "$dst"
    printf '%s\n' "$dst"
}

codex_promote_runtime_payload() {
    local src="$1" name target old
    [ -x "$src/codex" ] || return 1
    for name in codex codex-resources codex-path codex-package.json runtime-build.json; do
        [ -e "$src/$name" ] || return 1
    done
    for name in codex codex-resources codex-path codex-package.json runtime-build.json; do
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
    codex_prune_runtime_store
}

codex_repair_runtime_from_raw_unlocked() {
    local version package_spec
    codex_raw_integrity_ok || {
        codex_fail "cached raw package integrity check failed; run codex update"
        return 1
    }
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
    codex_validate_runtime_retention || return $?
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

codex_runtime_integrity_ok() {
    [ -r "$CODEX_NATIVE_RUNTIME_DIR/runtime-build.json" ] || return 1
    [ -x "$CODEX_NATIVE_RUNTIME_BUILDER" ] || return 1
    python3 - "$CODEX_NATIVE_RUNTIME" "$CODEX_NATIVE_RUNTIME_DIR/runtime-build.json" \
        "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_STATE_FILE" "$CODEX_NATIVE_PATCH_POLICY" <<'PY'
import hashlib, json, sys
from pathlib import Path

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

runtime, manifest_path, builder, state_path = map(Path, sys.argv[1:5])
policy = sys.argv[5]
try:
    manifest = json.loads(manifest_path.read_text())
    state = json.loads(state_path.read_text())
    runtime_sha = sha256(runtime)
    ok = (
        manifest.get("patch_policy") == policy
        and manifest.get("builder_sha256") == sha256(builder)
        and manifest.get("runtime_sha256") == runtime_sha
        and state.get("runtime_sha256") == runtime_sha
    )
except Exception:
    ok = False
raise SystemExit(0 if ok else 1)
PY
}

codex_raw_integrity_ok() {
    [ -x "$CODEX_NATIVE_RAW_VENDOR/bin/codex" ] || return 1
    python3 - "$CODEX_NATIVE_RAW_VENDOR/bin/codex" "$CODEX_NATIVE_STATE_FILE" <<'PY'
import hashlib, json, sys
from pathlib import Path

raw, state_path = map(Path, sys.argv[1:3])
try:
    expected = json.loads(state_path.read_text()).get("raw_sha256", "")
    digest = hashlib.sha256()
    with raw.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    ok = bool(expected and digest.hexdigest() == expected)
except Exception:
    ok = False
raise SystemExit(0 if ok else 1)
PY
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

codex_refresh_runtime_metadata() {
    local version raw_sha runtime_sha package_spec runtime_path tuple_id
    local wrapper_version wrapper_commit state_wrapper_version state_wrapper_commit active_tuple_id
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    package_spec="$(codex_read_state_field package_spec)"
    wrapper_version="$(codex_current_wrapper_version)"
    wrapper_commit="$(codex_current_wrapper_commit)"
    state_wrapper_version="$(codex_read_state_field wrapper_version)"
    state_wrapper_commit="$(codex_read_state_field wrapper_commit)"
    active_tuple_id="$(codex_read_state_field active_tuple_id)"
    [ -n "$version" ] && [ -n "$raw_sha" ] && [ -n "$runtime_sha" ] && [ -n "$package_spec" ] || return 0
    if [ -n "$active_tuple_id" ] &&
        [ "$state_wrapper_version" = "$wrapper_version" ] &&
        [ "$state_wrapper_commit" = "$wrapper_commit" ]; then
        return 0
    fi
    runtime_path="$CODEX_NATIVE_STORE_DIR/runtime/$(codex_store_id "$version" "$runtime_sha")"
    if [ ! -x "$runtime_path/codex" ] || [ ! -r "$runtime_path/runtime-build.json" ]; then
        runtime_path="$(codex_store_runtime_payload "$version" "$runtime_sha")"
    fi
    tuple_id="$(codex_record_registry "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$runtime_path")"
    codex_write_json_state "$version" "$raw_sha" "$runtime_sha" "$package_spec" "$tuple_id"
    codex_prune_runtime_store
}

codex_ensure_runtime_ready() {
    if codex_runtime_ok; then
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

codex_network_boundary_json() {
    local probe baseline off on reset baseline_status=0 off_status=0 on_status=0 reset_status=0
    mkdir -p "$CODEX_NATIVE_DOCTOR_DIR"
    probe="$CODEX_NATIVE_DOCTOR_DIR/network-probe.py"
    cat >"$probe" <<'PY'
import json, socket
from pathlib import Path

status = {}
for line in Path("/proc/self/status").read_text().splitlines():
    if line.startswith("NoNewPrivs:"):
        status["no_new_privs"] = int(line.split()[1])
    elif line.startswith("Seccomp:"):
        status["seccomp"] = int(line.split()[1])
try:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.close()
    status["socket_allowed"] = True
    status["socket_errno"] = None
except OSError as exc:
    status["socket_allowed"] = False
    status["socket_errno"] = exc.errno
print(json.dumps(status, sort_keys=True))
PY
    baseline="$(python3 "$probe" 2>/dev/null)" || baseline_status=$?
    off="$("$CODEX_NATIVE_RUNTIME" sandbox -c sandbox_workspace_write.network_access=false \
        python3 "$probe" 2>/dev/null)" || off_status=$?
    on="$("$CODEX_NATIVE_RUNTIME" sandbox \
        -c permissions.wrapper-network.network.enabled=true -P wrapper-network \
        python3 "$probe" 2>/dev/null)" || on_status=$?
    reset="$("$CODEX_NATIVE_RUNTIME" sandbox -c sandbox_workspace_write.network_access=false \
        python3 "$probe" 2>/dev/null)" || reset_status=$?
    python3 - "$baseline" "$off" "$on" "$reset" \
        "$baseline_status" "$off_status" "$on_status" "$reset_status" <<'PY'
import json, sys

names = ("baseline", "off", "on", "reset")
reports = {}
statuses = [int(value) for value in sys.argv[5:9]]
for name, raw, status in zip(names, sys.argv[1:5], statuses):
    try:
        reports[name] = json.loads(raw)
    except Exception:
        reports[name] = {"parse_error": True, "raw": raw, "exit_code": status}

baseline_ok = statuses[0] == 0 and reports["baseline"].get("socket_allowed") is True
off_ok = (
    statuses[1] == 0
    and reports["off"].get("socket_allowed") is False
    and reports["off"].get("socket_errno") == 1
    and reports["off"].get("no_new_privs") == 1
    and reports["off"].get("seccomp") == 2
)
on_ok = statuses[2] == 0 and reports["on"].get("socket_allowed") is True
reset_ok = (
    statuses[3] == 0
    and reports["reset"].get("socket_allowed") is False
    and reports["reset"].get("socket_errno") == 1
)
if not baseline_ok:
    overall = "inconclusive"
elif off_ok and on_ok and reset_ok:
    overall = "ok"
else:
    overall = "fail"
print(json.dumps({
    "overallStatus": overall,
    "checks": {
        "baseline_socket": baseline_ok,
        "network_off": off_ok,
        "network_on": on_ok,
        "network_reset": reset_ok,
    },
    "reports": reports,
}, ensure_ascii=True, sort_keys=True))
PY
}

codex_wrapper_doctor_json() {
    local version raw_sha runtime_sha network_json
    version="$(codex_read_state_field version)"
    raw_sha="$(codex_read_state_field raw_sha256)"
    runtime_sha="$(codex_read_state_field runtime_sha256)"
    codex_prepare_runtime_env
    network_json="$(codex_network_boundary_json)"
    python3 - "$CODEX_NATIVE_RUNTIME" "$CODEX_NATIVE_RUNTIME_DIR" "$CODEX_NATIVE_RAW_VENDOR" "$CODEX_NATIVE_RESOLV_CONF" "$CODEX_NATIVE_CERT_FILE" "$CODEX_NATIVE_STATE_FILE" "$CODEX_NATIVE_REGISTRY_FILE" "$version" "$raw_sha" "$runtime_sha" "$CODEX_NATIVE_PREFIX" "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_PATCH_POLICY" "$network_json" <<'PY'
import filecmp, hashlib, json, os, subprocess, sys
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
builder = Path(sys.argv[12])
patch_policy = sys.argv[13]
network = json.loads(sys.argv[14])
path_bwrap = runtime_dir / "codex-path/bwrap"
bundled_bwrap = runtime_dir / "codex-resources/bwrap"
rg = runtime_dir / "codex-path/rg"
manifest_path = runtime_dir / "runtime-build.json"

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

try:
    manifest = json.loads(manifest_path.read_text())
except Exception:
    manifest = {}
try:
    actual_raw_sha = sha256(raw_vendor / "bin/codex")
except Exception:
    actual_raw_sha = ""
try:
    actual_runtime_sha = sha256(runtime)
except Exception:
    actual_runtime_sha = ""
try:
    raw_bytes = (raw_vendor / "bin/codex").read_bytes()
    runtime_bytes = runtime.read_bytes()
    dns_only = raw_bytes.replace(b"/etc/resolv.conf", b"/proc/self/fd/33") == runtime_bytes
except Exception:
    dns_only = False
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
    "raw_hash": bool(actual_raw_sha and actual_raw_sha == raw_sha == manifest.get("raw_sha256")),
    "runtime_hash": bool(actual_runtime_sha and actual_runtime_sha == runtime_sha == manifest.get("runtime_sha256")),
    "build_manifest": bool(
        manifest.get("patch_policy") == patch_policy
        and manifest.get("builder_sha256") == sha256(builder)
    ),
    "dns_only_patch": dns_only,
    "network_boundary": network.get("overallStatus") != "fail",
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
    "schema": 3,
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
    "networkBoundary": network,
    "buildManifest": manifest,
    "checks": checks,
}, ensure_ascii=True, sort_keys=True))
PY
}

codex_wrapper_doctor() {
    if [ "${1:-}" = "--json" ]; then
        codex_wrapper_doctor_json
    else
        codex_wrapper_doctor_json | python3 -c '
import json, sys

data = json.load(sys.stdin)
checks = data.get("checks", {})
paths = data.get("paths", {})
network = data.get("networkBoundary", {})
manifest = data.get("buildManifest", {})
status = data.get("overallStatus", "fail")
use_color = sys.stdout.isatty() and not bool(__import__("os").environ.get("NO_COLOR"))
line = "─" * 61
counts = {"ok": 0, "idle": 0, "warn": 0, "fail": 0}

def color(code, text):
    return f"\033[{code}m{text}\033[0m" if use_color else text

def warn_mark():
    return color("33", "⚠")

def status_mark(item_status):
    if item_status == "ok":
        return color("32", "✓")
    if item_status == "warn":
        return color("33", "⚠")
    if item_status == "idle":
        return color("37", "○")
    return color("31", "✗")

def label(text):
    return color("36", text)

def section(text):
    print()
    print(color("1", text))

def row_status(item_status, name, summary):
    counts[item_status] += 1
    print("  {} {} {}".format(status_mark(item_status), label("{:<12}".format(name)), summary))

def row(ok, name, summary):
    row_status("ok" if ok else "fail", name, summary)

def detail(name, value):
    print(f"      {name:<24} {value}")

def probe_detail(name, item_status, value):
    counts[item_status] += 1
    print(f"      {name:<24} {value}")

def compact_hash(value):
    return value[:12] if value else "missing"

print("{} · {}".format(color("1", "Termux Wrapper Doctor"), data.get("version", "unknown")))
notes = []
if network.get("overallStatus") == "inconclusive":
    notes.append(("sandbox", "network boundary baseline was blocked by the outer environment; restricted probes still passed."))
if status != "ok":
    notes.append(("wrapper", "one or more wrapper checks failed."))
if notes:
    print()
    print(color("1", "Notes"))
    for name, summary in notes:
        print(f"   {warn_mark()} {label(name):<12} {summary}")

section("Runtime")
row(checks.get("runtime"), "runtime", "managed executable · {}".format(compact_hash(data.get("runtime_sha256", ""))))
detail("executable", paths.get("runtime", "missing"))
row(checks.get("raw"), "raw", "official linux-arm64 package · {}".format(compact_hash(data.get("raw_sha256", ""))))
detail("vendor", paths.get("raw_vendor", "missing"))
row(checks.get("build_manifest"), "manifest", manifest.get("patch_policy", "missing"))
detail("builder hash", compact_hash(manifest.get("builder_sha256", "")))
row(checks.get("dns_only_patch"), "patch", "DNS resolver path redirects to fd 33 only")
detail("changed bytes", manifest.get("changed_byte_count", "unknown"))
row(checks.get("runtime_hash") and checks.get("raw_hash"), "integrity", "runtime and raw hashes match recorded state")
detail("active tuple", data.get("activeTupleId", "missing"))

section("Support")
row(checks.get("path_bwrap") and checks.get("bundled_bwrap") and checks.get("bwrap_exec"), "bwrap", "Termux compatibility launcher is executable")
detail("launcher", "codex-path/bwrap")
row(checks.get("rg") and checks.get("rg_real") and checks.get("rg_exec"), "search", "ripgrep shim and original rg are executable")
detail("provider", "managed rg shim")
row(checks.get("support_bwrap_match") and checks.get("support_rg_match"), "support", "installed support files match wrapper files")
row(checks.get("zsh"), "zsh", "bundled zsh resource is present")

section("Environment")
row(checks.get("resolv"), "resolver", "fd 33 source is readable")
detail("source", "/proc/self/fd/33")
row(checks.get("cert"), "cert", "Termux CA bundle is readable")
row(checks.get("state") and checks.get("registry") and checks.get("registry_active_tuple"), "state", "state and registry point at active runtime")
detail("state", paths.get("state", "missing"))
detail("registry", paths.get("registry", "missing"))

section("Sandbox")
net_status = network.get("overallStatus", "missing")
if net_status == "ok":
    network_row_status = "ok"
elif net_status == "inconclusive":
    network_row_status = "warn"
else:
    network_row_status = "fail"
row_status(network_row_status, "network", f"boundary probe {net_status}")
for name in ("baseline_socket", "network_off", "network_on", "network_reset"):
    probe_ok = bool(network.get("checks", {}).get(name, False))
    probe_status = "ok" if probe_ok else ("warn" if net_status == "inconclusive" else "fail")
    probe_detail(name.replace("_", " "), probe_status, probe_ok)
print()
summary_status = "fail" if counts["fail"] else ("degraded" if counts["warn"] else "ok")
summary = "{ok} ok · {idle} idle · {warn} warn · {fail} fail {status}".format(
    ok=counts["ok"],
    idle=counts["idle"],
    warn=counts["warn"],
    fail=counts["fail"],
    status=summary_status,
)
if summary_status == "ok":
    print(color("32", summary))
elif summary_status == "degraded":
    print(color("33", summary))
else:
    print(color("31", summary))
raise SystemExit(0 if status == "ok" else 1)
'
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
    python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$latest" "$CODEX_NATIVE_STORE_DIR/runtime" "$interactive_limit" "$mode" \
        "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_PATCH_POLICY" <<'PY'
import hashlib, json, os, sys
from pathlib import Path
path = Path(sys.argv[1])
latest = sys.argv[2]
store_runtime_root = Path(sys.argv[3]).resolve()
interactive_limit = int(sys.argv[4])
mode = sys.argv[5]
builder = Path(sys.argv[6])
policy = sys.argv[7]
data = json.loads(path.read_text()) if path.exists() else {"installs": []}
active_tuple_id = data.get("active_tuple_id", "")
builder_sha = hashlib.sha256(builder.read_bytes()).hexdigest()

def managed_runtime_path(value: str) -> bool:
    if not value:
        return False
    try:
        runtime_path = Path(value).resolve()
    except Exception:
        return False
    if not ((runtime_path / "codex").exists() and store_runtime_root in runtime_path.parents):
        return False
    try:
        manifest = json.loads((runtime_path / "runtime-build.json").read_text())
        digest = hashlib.sha256((runtime_path / "codex").read_bytes()).hexdigest()
        return (
            manifest.get("patch_policy") == policy
            and manifest.get("builder_sha256") == builder_sha
            and manifest.get("runtime_sha256") == digest
        )
    except Exception:
        return False

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
    selected="$(python3 - "$CODEX_NATIVE_REGISTRY_FILE" "$choice" "$latest" "$CODEX_NATIVE_STORE_DIR/runtime" \
        "$CODEX_NATIVE_RUNTIME_BUILDER" "$CODEX_NATIVE_PATCH_POLICY" <<'PY'
import hashlib, json, sys
from pathlib import Path
path = Path(sys.argv[1])
choice = sys.argv[2]
latest = sys.argv[3]
store_runtime_root = Path(sys.argv[4]).resolve()
builder = Path(sys.argv[5])
policy = sys.argv[6]
builder_sha = hashlib.sha256(builder.read_bytes()).hexdigest()
data = json.loads(path.read_text()) if path.exists() else {"installs": []}

def managed_runtime_path(value: str) -> bool:
    if not value:
        return False
    try:
        runtime_path = Path(value).resolve()
    except Exception:
        return False
    if not ((runtime_path / "codex").exists() and store_runtime_root in runtime_path.parents):
        return False
    try:
        manifest = json.loads((runtime_path / "runtime-build.json").read_text())
        digest = hashlib.sha256((runtime_path / "codex").read_bytes()).hexdigest()
        return (
            manifest.get("patch_policy") == policy
            and manifest.get("builder_sha256") == builder_sha
            and manifest.get("runtime_sha256") == digest
        )
    except Exception:
        return False

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
        codex_prune_runtime_store
        codex_say "using Codex $version"
    fi
    codex_version
}

codex_bootstrap_store() {
    local version raw_sha runtime_sha package_spec runtime_path tuple_id
    [ -n "$(codex_read_state_field active_tuple_id)" ] && return 0
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
            codex_public_doctor "$@"
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
