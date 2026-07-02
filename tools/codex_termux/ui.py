"""Structured UI text for the Codex Termux wrapper."""

from __future__ import annotations

from .errors import IntegrityError


def _arg(args: tuple[str, ...], index: int, default: str = "") -> str:
    return args[index] if index < len(args) else default


def format_text(kind: str, value: str = "", *, color: bool = False) -> str:
    if kind == "dim" or kind == "prompt":
        return _color("2", value, color)
    if kind == "number":
        return _color("36", f"{value:>2}.", color)
    if kind == "separator":
        width = int(value or "61")
        return _color("2", "─" * width, color)
    if kind == "display-version":
        return value.removesuffix("-linux-arm64")
    if kind == "badge":
        text, code = _badge(value)
        return _color(code, text, color)
    raise IntegrityError(f"unknown UI format kind: {kind}")


def _color(code: str, text_value: str, enabled: bool) -> str:
    if enabled:
        return f"\033[{code}m{text_value}\033[0m"
    return text_value


def _badge(kind: str) -> tuple[str, str]:
    badges = {
        "active": (" 🟢 active ", "42;30"),
        "current": (" 🟢 current ", "42;30"),
        "cached": (" 📦 cached ", "44;97"),
        "run": (" ▶ run ", "46;30"),
        "install": (" ⬇ install ", "43;30"),
        "update": (" ⬇ update ", "43;30"),
        "latest": (" ⬆ latest ", "45;97"),
        "recent": (" 🕘 recent ", "46;30"),
        "keep": (" ↵ keep ", "100;97"),
    }
    return badges.get(kind, (f" {kind} ", "2"))


def text(key: str, *args: str) -> str:
    messages = {
        "selection_cancelled": "Selection cancelled.",
        "profile_create_cancelled": "Profile creation cancelled.",
        "choose_profile_title": "Choose profile",
        "choose_profile_subtitle": "Select CODEX_HOME target",
        "choose_profile_prompt": "Choose profile > ",
        "choose_profile_more": "  (More options: codex termux profile NAME)",
        "choose_runtime_prompt": "Choose runtime > ",
        "update_complete_title": "Update complete",
        "update_available_title": "Update available",
        "launch_now_prompt": "Launch now [y/N]> ",
        "apply_update_prompt": "Apply update [y/N]> ",
        "launch_label": "launch Codex",
        "done_label": "done",
        "profile_create_cancelled": "Profile creation cancelled.",
        "restored_verified": "Restored the active runtime from the verified copy.",
        "setup_reserved": (
            "The upstream setup command is reserved. Use codex termux install, "
            "update, repair, or notify for wrapper operations."
        ),
        "doctor_wrapper_title": "Wrapper doctor",
        "session_stub": "Use codex termux session for the cross-profile session picker.",
    }
    if key in messages:
        return messages[key]
    if key == "update_ready_subtitle":
        return f"Codex {_arg(args, 0)} is ready"
    if key == "current_kept":
        return f"Kept current runtime ({_arg(args, 0)})."
    if key == "create_profile_prompt":
        return f"Create profile '{_arg(args, 0)}' [y/N]> "
    if key == "created_profile":
        return f"Created profile {_arg(args, 0)}."
    if key == "installed_codex":
        return f"Installed Codex {_arg(args, 0)}"
    if key == "rebuilt_cached_runtime":
        return f"Rebuilt runtime from cached raw package ({_arg(args, 0)})"
    if key == "update_failed_continue":
        return f"Update failed. Continuing with {_arg(args, 0)}."
    if key == "restored_backup":
        return f"Restored {_arg(args, 0)} from {_arg(args, 1)}."
    if key == "removed_runtime":
        return f"Removed the managed runtime. State remains at {_arg(args, 0)}."
    if key == "invalid_profile":
        return f"Invalid profile name: {_arg(args, 0)}"
    if key == "missing_profile":
        return f"Profile does not exist: {_arg(args, 0)}"
    if key == "profile_arg_error":
        return f"Profile {_arg(args, 0)} does not take arguments"
    raise IntegrityError(f"unknown UI text key: {key}")


def step_text(key: str, *args: str) -> str:
    messages = {
        "validate_archive": "Validating package archive",
        "unpack_archive": "Unpacking package archive",
        "stage_raw": "Staging raw package",
        "build_runtime": "Building patched runtime",
        "assemble_runtime": "Assembling runtime bundle",
        "smoke_test_runtime": "Smoke-testing runtime",
        "activate_runtime": "Activating runtime",
        "install_runtime": "Installing wrapper support and fresh upstream runtime",
        "rebuild_runtime": "Rebuilding wrapper support with cached raw runtime",
        "repair_runtime": "Repairing runtime from the cached raw package",
        "repair_support": "Repairing wrapper support and launcher",
        "repair_metadata": "Repairing runtime metadata",
        "rebuild_cached_runtime": "Rebuilding runtime from the cached raw package",
    }
    if key in messages:
        return messages[key]
    if key == "fetch_package":
        return f"Fetching {_arg(args, 0)}"
    if key == "update_runtime":
        return f"Updating Codex {_arg(args, 0)} -> {_arg(args, 1)}"
    if key == "switch_runtime":
        return f"Switching to Codex {_arg(args, 0)}"
    if key == "launch_codex":
        return f"Launching Codex {_arg(args, 0)}"
    if key == "open_profile":
        return f"Opening profile {_arg(args, 0)}"
    raise IntegrityError(f"unknown UI step key: {key}")
