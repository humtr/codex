#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codex-notify-baseline.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'notify-baseline: FAIL: %s\n' "$*" >&2
    exit 1
}

case_index=0
run_payload() {
    local payload="$1" event="${2:-Stop}" case_dir
    case_index=$((case_index + 1))
    case_dir="$TMP_DIR/case-$case_index"
    mkdir -p "$case_dir/home" "$case_dir/state/notify" "$case_dir/tmp"
    printf '%s' "$payload" | \
        CODEX_TERMUX_HOME="$case_dir/home" \
        CODEX_TERMUX_STATE_DIR="$case_dir/state" \
        CODEX_TERMUX_TMPDIR="$case_dir/tmp" \
        CODEX_TERMUX_NOTIFY_NO_API=1 \
        bash "$ROOT_DIR/tools/codex-turn-notify.sh" --event "$event" >/dev/null 2>&1
    printf '%s\n' "$case_dir/state/notify/last-payload.json"
}

assert_payload() {
    local path="$1" expected_title="$2" expected_content="$3"
    python3 -B - "$path" "$expected_title" "$expected_content" <<'PYTHON'
import json
import sys

path, expected_title, expected_content = sys.argv[1:]
payload = json.load(open(path, encoding="utf-8"))
assert payload["title"] == expected_title, (payload["title"], expected_title)
assert payload["content"] == expected_content, (payload["content"], expected_content)
PYTHON
}

path="$(run_payload '{"title":"Explicit","message":"one line"}')"
assert_payload "$path" "Explicit" $'one line\n'

path="$(run_payload '{"project_name":"Project Alpha","message":"done"}')"
assert_payload "$path" "Codex: Project Alpha" $'done\n'

path="$(run_payload '{"repositoryName":"repository-beta","message":"done"}')"
assert_payload "$path" "Codex: repository-beta" $'done\n'

ssh_repo="$TMP_DIR/ssh-origin"
mkdir -p "$ssh_repo"
git -C "$ssh_repo" init -q
git -C "$ssh_repo" remote add origin git@github.com:example/ssh-project.git
path="$(run_payload "{\"cwd\":\"$ssh_repo\",\"message\":\"done\"}")"
assert_payload "$path" "Codex: ssh-project" $'done\n'

https_repo="$TMP_DIR/https-origin"
mkdir -p "$https_repo"
git -C "$https_repo" init -q
git -C "$https_repo" remote add origin https://github.com/example/https-project.git
path="$(run_payload "{\"cwd\":\"$https_repo\",\"message\":\"done\"}")"
assert_payload "$path" "Codex: https-project" $'done\n'

no_origin="$TMP_DIR/no-origin-project"
mkdir -p "$no_origin"
git -C "$no_origin" init -q
path="$(run_payload "{\"cwd\":\"$no_origin\",\"message\":\"done\"}")"
assert_payload "$path" "Codex: no-origin-project" $'done\n'

non_git="$TMP_DIR/non-git"
mkdir -p "$non_git"
path="$(run_payload "{\"cwd\":\"$non_git\",\"message\":\"done\"}")"
assert_payload "$path" "Codex: General" $'done\n'

one_to_ten="$(python3 -B - <<'PYTHON'
import json
print(json.dumps({"message": "\n".join(f"line {i}" for i in range(1, 11))}))
PYTHON
)"
path="$(run_payload "$one_to_ten")"
assert_payload "$path" "Codex: General" "$(printf 'line %s\n' {1..10} | sed -z 's/\n$//')"

one_to_eleven="$(python3 -B - <<'PYTHON'
import json
print(json.dumps({"message": "\n".join(f"line {i}" for i in range(1, 12))}))
PYTHON
)"
path="$(run_payload "$one_to_eleven")"
python3 -B - "$path" <<'PYTHON'
import json
import sys
payload = json.load(open(sys.argv[1], encoding="utf-8"))
assert payload["content"].splitlines() == [f"line {i}" for i in range(1, 11)]
PYTHON

crlf_payload="$(python3 -B - <<'PYTHON'
import json
print(json.dumps({"message": "alpha\r\nbeta\rgamma"}))
PYTHON
)"
path="$(run_payload "$crlf_payload")"
assert_payload "$path" "Codex: General" $'alpha\nbeta\ngamma'

provider="$TMP_DIR/provider"
mkdir -p "$provider/bin" "$provider/state/notify" "$provider/home" "$provider/tmp"
cat >"$provider/bin/termux-toast" <<'SH'
#!/bin/sh
printf '%s\n' "$*" >"$CODEX_TOAST_ARGS"
SH
chmod 755 "$provider/bin/termux-toast"

printf '%s' '{"title":"Toast","content":"body"}' | \
    env -i \
        HOME="$provider/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$provider/bin:$PATH" \
        CODEX_TOAST_ARGS="$provider/short.args" \
        CODEX_TERMUX_HOME="$provider/home" \
        CODEX_TERMUX_STATE_DIR="$provider/state" \
        CODEX_TERMUX_TMPDIR="$provider/tmp" \
        CODEX_TERMUX_NOTIFY_CHANNEL=toast \
        CODEX_TERMUX_NOTIFY_TOAST_SHORT=1 \
        bash "$ROOT_DIR/tools/termux-notify.sh" >/dev/null 2>&1

grep -F -- '-s' "$provider/short.args" >/dev/null || fail 'short toast did not pass -s'

printf '%s' '{"title":"Toast","content":"body"}' | \
    env -i \
        HOME="$provider/home" \
        PREFIX="${PREFIX:-/data/data/com.termux/files/usr}" \
        PATH="$provider/bin:$PATH" \
        CODEX_TOAST_ARGS="$provider/long.args" \
        CODEX_TERMUX_HOME="$provider/home" \
        CODEX_TERMUX_STATE_DIR="$provider/state" \
        CODEX_TERMUX_TMPDIR="$provider/tmp" \
        CODEX_TERMUX_NOTIFY_CHANNEL=toast \
        CODEX_TERMUX_NOTIFY_TOAST_SHORT=0 \
        bash "$ROOT_DIR/tools/termux-notify.sh" >/dev/null 2>&1

if grep -F -- '-s' "$provider/long.args" >/dev/null; then
    fail 'long toast unexpectedly passed -s'
fi

printf 'notify-baseline: ok\n'
