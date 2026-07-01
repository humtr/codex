#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'notify-model: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - <<'PYTHON' || fail 'notify model failed'
from codex_termux import notify


assert notify.canonical_hook("stop") == "Stop"
assert notify.canonical_hook("pretooluse") == "PreToolUse"
assert notify.hook_valid("SubagentStop")
assert not notify.hook_valid("TypoHook")
assert notify.normalize_hooks("stop,Stop,SubagentStop") == "Stop,SubagentStop"
assert notify.normalize_hooks("all") == "all"
assert notify.hook_list("all")[0] == "SessionStart"
assert notify.hook_list("Stop") == ["Stop"]
assert notify.status_message("Stop") == "Notify turn completion"

settings = notify.NotifySettings(
    content_chars="0",
    preserve_newlines="1",
    toast_gravity="top",
    toast_short="0",
    toast_background="",
    toast_color="",
    group="codex-turns",
    channel="both",
    hooks="stop,SubagentStop",
    pretooluse="1",
)
env_text = notify.render_config_env(settings)
assert "CODEX_TERMUX_NOTIFY_CHANNEL=both\n" in env_text
assert "CODEX_TERMUX_NOTIFY_HOOKS=Stop,SubagentStop\n" in env_text
system_text = notify.render_system_config(hooks="Stop", turn_notify="/tmp/notify")
assert "[[hooks.Stop]]" in system_text
assert "Notify turn completion" in system_text

for bad in (
    settings.__class__(**{**settings.__dict__, "channel": "invalid"}),
    settings.__class__(**{**settings.__dict__, "toast_gravity": "center"}),
    settings.__class__(**{**settings.__dict__, "hooks": "TypoHook"}),
):
    try:
        notify.render_config_env(bad)
    except notify.NotifyConfigError:
        pass
    else:
        raise AssertionError(bad)
PYTHON

normalized="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli notify-hook --action normalize --value stop,SubagentStop
)"
[ "$normalized" = "Stop,SubagentStop" ] || fail "normalize mismatch: $normalized"

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
    python3 -B -m codex_termux.cli notify-config-env \
        --content-chars 140 \
        --preserve-newlines 0 \
        --toast-gravity center \
        --toast-short 0 \
        --toast-background "" \
        --toast-color "" \
        --group codex-turns \
        --channel both \
        --hooks Stop \
        --pretooluse 0 >/dev/null 2>&1
then
    fail 'invalid toast gravity was accepted'
fi

printf 'notify-model: ok\n'
