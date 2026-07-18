#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'notify-model: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - <<'PYTHON' \
    || fail 'notify model failed'
from wrapper.notification import hooks as notify
from wrapper.notification.config import load_settings

assert notify.canonical_hook("stop") == "Stop"
assert notify.canonical_hook("pretooluse") == "PreToolUse"
assert notify.hook_valid("SubagentStop")
assert not notify.hook_valid("TypoHook")
assert notify.normalize_hooks("stop,Stop,SubagentStop") == "Stop,SubagentStop"
assert notify.normalize_hooks("all") == "all"
assert notify.hook_list("all")[0] == "SessionStart"
assert notify.hook_list("Stop") == ["Stop"]
assert notify.parse_hook_selection("") == "Stop"
assert notify.parse_hook_selection("1") == "SessionStart"
assert notify.parse_hook_selection("1 Stop") == "SessionStart,Stop"
assert notify.parse_hook_selection("0") == "all"
assert notify.parse_channel_selection("") == "both"
assert notify.parse_channel_selection("1") == "notification"
assert notify.parse_channel_selection("2") == "toast"
assert notify.parse_channel_selection("3") == "both"
assert notify.parse_channel_selection("toast") == "toast"
assert not notify.channel_needs_gravity("notification")
assert notify.channel_needs_gravity("both")
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

for selection in ("99", "1abc"):
    try:
        notify.parse_hook_selection(selection)
    except notify.NotifyConfigError:
        pass
    else:
        raise AssertionError(selection)

assert load_settings(
    {
        "CODEX_TERMUX_NOTIFY_CHANNEL": "both",
        "CODEX_TERMUX_NOTIFY_TOAST_DURATION": "short",
        "CODEX_TERMUX_NOTIFY_TOAST_SHORT": "0",
    }
).toast_duration == "short"
assert load_settings({"CODEX_TERMUX_NOTIFY_TOAST_SHORT": "1"}).toast_duration == "short"
assert load_settings({"CODEX_TERMUX_NOTIFY_TOAST_SHORT": "0"}).toast_duration == "long"
PYTHON

normalized="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli notify-hook --action normalize --value stop,SubagentStop
)"
[ "$normalized" = "Stop,SubagentStop" ] || fail "normalize mismatch: $normalized"

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
    python3 -B -m wrapper.cli notify-config-env \
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

selection="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli notify-hook --action parse-selection --value "1 Stop"
)"
[ "$selection" = "SessionStart,Stop" ] || fail "selection mismatch: $selection"

channel="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli notify-channel --action parse --value 2
)"
[ "$channel" = "toast" ] || fail "channel mismatch: $channel"

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
    python3 -B -m wrapper.cli notify-channel --action needs-gravity --value both \
    || fail 'both channel did not require gravity'

if PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
    python3 -B -m wrapper.cli notify-channel --action needs-gravity --value notification
then
    fail 'notification channel required gravity'
fi

command_env="$(
    CODEX_TERMUX_NOTIFY_CONFIG=/tmp/notify.env \
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli notify-command-config \
            --field config-env -- --channel both --hook stop --hook SubagentStop
)"
case "$command_env" in
    *"CODEX_TERMUX_NOTIFY_CHANNEL=both"*) ;;
    *) fail "notify command channel mismatch: $command_env" ;;
esac
case "$command_env" in
    *"CODEX_TERMUX_NOTIFY_HOOKS=Stop,SubagentStop"*) ;;
    *) fail "notify command config env mismatch: $command_env" ;;
esac

legacy="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src:$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli notify-hook --action normalize --value stop
)"
[ "$legacy" = "Stop" ] || fail "legacy notify CLI mismatch: $legacy"

printf 'notify-model: ok\n'
