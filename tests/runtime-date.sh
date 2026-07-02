#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$TMP_PARENT/codex-runtime-date-test.$$"
trap 'rm -rf "$TMP_DIR"' EXIT
mkdir -p "$TMP_DIR"

fail() {
    printf 'runtime-date: FAIL: %s\n' "$*" >&2
    exit 1
}

date_text="$(
    printf '{"0.142.5":"2026-07-01T02:03:04.000Z"}' |
        PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
            python3 -B -m codex_termux.cli upstream-release-date --version 0.142.5
)"
[ "$date_text" = "2026-07-01" ] || fail "release date parse mismatch: $date_text"

display_date="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli display-runtime-date --value 20260702T030405Z
)"
[ "$display_date" = "2026-07-02" ] || fail "display date mismatch: $display_date"

upstream_version="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli upstream-version --text "codex-cli 0.142.5"
)"
[ "$upstream_version" = "0.142.5" ] || fail "upstream version mismatch: $upstream_version"
upstream_version="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli upstream-version --text ""
)"
[ "$upstream_version" = "unknown" ] || fail "empty upstream version mismatch: $upstream_version"

pack_json="$TMP_DIR/pack.json"
printf '[{"filename":"codex-test.tgz","version":"0.142.5"}]\n' >"$pack_json"
package_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli package-fields-env --json-file "$pack_json"
)"
eval "$package_env"
[ "$CODEX_PACKAGE_FILENAME" = "codex-test.tgz" ] || fail "package filename env mismatch: $CODEX_PACKAGE_FILENAME"
[ "$CODEX_PACKAGE_VERSION" = "0.142.5" ] || fail "package version env mismatch: $CODEX_PACKAGE_VERSION"

version_report="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli version-report \
            --upstream "codex-cli 0.142.5" \
            --upstream-date "2026-07-01" \
            --runtime-date "2026-07-02" \
            --wrapper-version "260702-47" \
            --wrapper-commit "abcdef123456"
)"
case "$version_report" in
    $'codex-cli 0.142.5 (2026-07-01)\nruntime   2026-07-02\nwrapper   260702-47 (abcdef123456)') ;;
    *) fail "version report mismatch: $version_report" ;;
esac

cache_file="$TMP_DIR/upstream-cache.tsv"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli upstream-release-cache-write \
    --cache "$cache_file" --version 0.1.0 --release-date 2026-07-02
cache_value="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli upstream-release-cache-read \
        --cache "$cache_file" --version 0.1.0
)"
[ "$cache_value" = "2026-07-02" ] || fail "upstream cache read mismatch: $cache_value"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli upstream-release-cache-write \
    --cache "$cache_file" --version 0.1.0 --release-date 2026-07-03
cache_value="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli upstream-release-cache-read \
        --cache "$cache_file" --version 0.1.0
)"
[ "$cache_value" = "2026-07-03" ] || fail "upstream cache update mismatch: $cache_value"
if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli upstream-release-cache-read \
    --cache "$cache_file" --version missing >/dev/null; then
    fail "upstream cache missing version unexpectedly succeeded"
fi

package_spec="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli package-spec \
            --requested 0.142.5 \
            --default @openai/codex@latest-linux-arm64
)"
[ "$package_spec" = "@openai/codex@0.142.5-linux-arm64" ] || fail "package spec mismatch: $package_spec"

explicit_package_spec="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli package-spec \
            --requested @openai/codex@0.142.5-linux-arm64 \
            --default @openai/codex@latest-linux-arm64
)"
[ "$explicit_package_spec" = "@openai/codex@0.142.5-linux-arm64" ] || fail "explicit package spec mismatch: $explicit_package_spec"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-retention-ok --value 3 \
    || fail 'runtime retention rejected positive integer'

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli runtime-retention-ok --value 0
then
    fail 'runtime retention accepted zero'
fi

manager_dir="$TMP_DIR/manager"
runtime_dir="$TMP_DIR/runtime"
mkdir -p "$manager_dir" "$runtime_dir"
printf 'test\n' >"$runtime_dir/bwrap-termux-compat.py"
printf 'test\n' >"$runtime_dir/rg-termux-shim.sh"
support_dir="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli support-source-dir \
            --manager-dir "$manager_dir" \
            --runtime-dir "$runtime_dir"
)"
[ "$support_dir" = "$runtime_dir" ] || fail "support source runtime fallback mismatch: $support_dir"

printf 'test\n' >"$manager_dir/bwrap-termux-compat.py"
printf 'test\n' >"$manager_dir/rg-termux-shim.sh"
support_dir="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli support-source-dir \
            --manager-dir "$manager_dir" \
            --runtime-dir "$runtime_dir"
)"
[ "$support_dir" = "$manager_dir" ] || fail "support source manager priority mismatch: $support_dir"

cat >"$runtime_dir/wrapper-version.env" <<'ENV'
CODEX_TERMUX_WRAPPER_VERSION=runtime-version
CODEX_TERMUX_WRAPPER_COMMIT=runtime-commit
ENV
cat >"$manager_dir/wrapper-version.env" <<'ENV'
CODEX_TERMUX_WRAPPER_VERSION=manager-version
CODEX_TERMUX_WRAPPER_COMMIT=manager-commit
ENV
metadata_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli wrapper-metadata-env \
            --manager-dir "$manager_dir" \
            --runtime-dir "$runtime_dir"
)"
eval "$metadata_env"
[ "$CODEX_WRAPPER_VERSION" = "manager-version" ] || fail "wrapper metadata env manager version mismatch: $CODEX_WRAPPER_VERSION"
[ "$CODEX_WRAPPER_COMMIT" = "manager-commit" ] || fail "wrapper metadata env manager commit mismatch: $CODEX_WRAPPER_COMMIT"

rm -f "$manager_dir/wrapper-version.env"
metadata_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli wrapper-metadata-env \
            --manager-dir "$manager_dir" \
            --runtime-dir "$runtime_dir"
)"
eval "$metadata_env"
[ "$CODEX_WRAPPER_VERSION" = "runtime-version" ] || fail "wrapper metadata env version mismatch: $CODEX_WRAPPER_VERSION"
[ "$CODEX_WRAPPER_COMMIT" = "runtime-commit" ] || fail "wrapper metadata env commit mismatch: $CODEX_WRAPPER_COMMIT"

mode="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-mode --mode always
)"
[ "$mode" = "force" ] || fail "auto-update mode mismatch: $mode"

decision="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli update-prompt-decision --choice y
)"
[ "$decision" = "apply" ] || fail "update prompt apply mismatch: $decision"

decision="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli update-prompt-decision --choice N
)"
[ "$decision" = "keep" ] || fail "update prompt keep mismatch: $decision"

decision="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli update-prompt-decision --choice ""
)"
[ "$decision" = "cancel" ] || fail "update prompt cancel mismatch: $decision"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli auto-update-due \
        --enabled 1 --mode prompt --now 100 --last 10 --interval 60 \
    || fail 'auto-update due rejected due update'

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli auto-update-due \
        --enabled 1 --mode off --now 100 --last 10 --interval 60
then
    fail 'auto-update due accepted off mode'
fi

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli failed-auto-update-due \
        --record $'0.142.4\t95' --version 0.142.4 --now 100 --interval 60
then
    fail 'failed auto-update retry accepted too early'
fi

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli failed-auto-update-due \
        --record $'0.142.4\t30' --version 0.142.4 --now 100 --interval 60 \
    || fail 'failed auto-update retry rejected due retry'

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-check-plan-env \
            --enabled 1 --mode prompt --current 0.142.4 --pending 0.142.5 \
            --now 100 --last 95 --interval 60
)"
eval "$plan_env"
[ "$CODEX_AUTO_UPDATE_ACTION" = use_pending ] || fail "pending auto-update plan action mismatch: $CODEX_AUTO_UPDATE_ACTION"
[ "$CODEX_AUTO_UPDATE_LATEST" = 0.142.5 ] || fail "pending auto-update latest mismatch: $CODEX_AUTO_UPDATE_LATEST"
[ "$CODEX_AUTO_UPDATE_CLEAR_PENDING" = 0 ] || fail "pending auto-update clear mismatch: $CODEX_AUTO_UPDATE_CLEAR_PENDING"

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-check-plan-env \
            --enabled 1 --mode prompt --current 0.142.4 --pending 0.142.4 \
            --now 100 --last 10 --interval 60
)"
eval "$plan_env"
[ "$CODEX_AUTO_UPDATE_ACTION" = fetch ] || fail "due auto-update plan action mismatch: $CODEX_AUTO_UPDATE_ACTION"
[ "$CODEX_AUTO_UPDATE_CLEAR_PENDING" = 1 ] || fail "stale pending clear mismatch: $CODEX_AUTO_UPDATE_CLEAR_PENDING"

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-check-plan-env \
            --enabled 1 --mode off --current 0.142.4 --pending 0.142.5 \
            --now 100 --last 10 --interval 60
)"
eval "$plan_env"
[ "$CODEX_AUTO_UPDATE_ACTION" = skip ] || fail "off auto-update plan action mismatch: $CODEX_AUTO_UPDATE_ACTION"

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-apply-plan-env \
            --current 0.142.4 --latest 0.142.5 --failed-record "" \
            --mode force --now 100 --interval 60
)"
eval "$plan_env"
[ "$CODEX_AUTO_UPDATE_ACTION" = install ] || fail "force auto-update apply action mismatch: $CODEX_AUTO_UPDATE_ACTION"

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-apply-plan-env \
            --current 0.142.4 --latest 0.142.5 --failed-record $'0.142.5\t95' \
            --mode prompt --now 100 --interval 60
)"
eval "$plan_env"
[ "$CODEX_AUTO_UPDATE_ACTION" = skip ] || fail "recent failed auto-update apply action mismatch: $CODEX_AUTO_UPDATE_ACTION"

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli auto-update-apply-plan-env \
            --current 0.142.4 --latest 0.142.4 --failed-record "" \
            --mode prompt --now 100 --interval 60
)"
eval "$plan_env"
[ "$CODEX_AUTO_UPDATE_ACTION" = clear_pending ] || fail "current auto-update apply action mismatch: $CODEX_AUTO_UPDATE_ACTION"

printf 'runtime-date: ok\n'
