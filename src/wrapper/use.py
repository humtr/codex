"""Cached runtime listing and selection rendering."""

from __future__ import annotations

import os
import shlex
import sys
from pathlib import Path

from . import registry
from .errors import SchemaError


MENU_TITLE = "Choose runtime"
MENU_SUBTITLE = "Select a managed Codex runtime"
MENU_MORE = "  (More options: codex termux use <version>)"


def render_runtime_rows(
    rows: list[dict[str, str]],
    *,
    mode: str,
    interactive_limit: int,
) -> int:
    if mode == "list":
        _render_list(rows)
        return 0
    _render_menu(rows, interactive_limit)
    return 0


def selection_plan_exports(row: dict[str, str]) -> str:
    action = "install_upstream" if row.get("kind") == "remote" else "activate_cached"
    data = {
        "CODEX_USE_PLAN_ACTION": action,
        "CODEX_USE_PLAN_RUNTIME_PATH": row.get("runtime_path", ""),
        "CODEX_USE_PLAN_RAW_PATH": row.get("raw_path", ""),
        "CODEX_USE_PLAN_VERSION": row.get("version", "unknown"),
        "CODEX_USE_PLAN_RAW_SHA256": row.get("raw_sha256", ""),
        "CODEX_USE_PLAN_RUNTIME_SHA256": row.get("runtime_sha256", ""),
        "CODEX_USE_PLAN_PACKAGE_SPEC": row.get("package_spec", ""),
    }
    return "\n".join(f"{key}={shlex.quote(value)}" for key, value in data.items())


def command_plan_exports(args: list[str]) -> str:
    first = args[0] if args else ""
    if not first:
        action = "menu"
        choice = ""
    elif first == "--list":
        action = "list"
        choice = ""
    else:
        action = "select"
        choice = first
    data = {
        "CODEX_USE_COMMAND_ACTION": action,
        "CODEX_USE_COMMAND_CHOICE": choice,
    }
    return "\n".join(f"{key}={shlex.quote(value)}" for key, value in data.items())


def _render_list(rows: list[dict[str, str]]) -> None:
    if not rows:
        raise SchemaError("no cached runtimes")
    latest_row, remaining = _menu_rows(rows)
    if latest_row is not None:
        print(
            "\t".join(
                [
                    "0",
                    latest_row["kind"],
                    latest_row.get("version", "unknown"),
                    latest_row.get("runtime_sha256", "")[:12],
                    latest_row.get("package_spec", ""),
                    latest_row.get("runtime_path", ""),
                    latest_row.get("created_at", ""),
                    latest_row.get("wrapper_version", ""),
                    latest_row.get("wrapper_commit", ""),
                ]
            )
        )
    for index, row in enumerate(remaining, 1):
        print(
            "\t".join(
                [
                    str(index),
                    row["kind"],
                    row.get("version", "unknown"),
                    row.get("runtime_sha256", "")[:12],
                    row.get("package_spec", ""),
                    row.get("runtime_path", ""),
                    row.get("created_at", ""),
                    row.get("wrapper_version", ""),
                    row.get("wrapper_commit", ""),
                ]
            )
        )


def _render_menu(rows: list[dict[str, str]], interactive_limit: int) -> None:
    if not rows:
        raise SchemaError("no cached runtimes")
    color_enabled = sys.stderr.isatty() and not os.environ.get("NO_COLOR")
    latest_row, remaining = _menu_rows(rows)
    count = 0
    truncated = False
    print(MENU_TITLE, file=sys.stderr)
    print(_dim(MENU_SUBTITLE, color_enabled), file=sys.stderr)
    if latest_row is not None:
        print(
            _menu_line("0", latest_row, color_enabled),
            file=sys.stderr,
        )
    for row in remaining:
        if interactive_limit and count >= interactive_limit:
            truncated = True
            continue
        count += 1
        print(_menu_line(str(count), row, color_enabled), file=sys.stderr)
    if truncated:
        print(_dim(MENU_MORE, color_enabled), file=sys.stderr)
    print(file=sys.stderr)
    print(count)


def _menu_rows(rows: list[dict[str, str]]) -> tuple[dict[str, str] | None, list[dict[str, str]]]:
    latest = next((row for row in rows if row.get("kind") == "remote"), None)
    if latest is not None:
        latest["menu_latest"] = "1"
    for row in rows:
        if row is not latest:
            row.pop("menu_latest", None)
    remaining = [row for row in rows if row is not latest]
    return latest, remaining


def _menu_line(index: str, row: dict[str, str], color_enabled: bool) -> str:
    label = registry.display_version(row.get("version", "unknown"))
    parts = [f"  {_number(index, color_enabled)} {_label(label, row, color_enabled)}"]
    badges = _badges(row, color_enabled)
    if badges:
        parts.append(badges)
    return " ".join(parts)


def _label(label: str, row: dict[str, str], color_enabled: bool) -> str:
    if row.get("kind") == "remote":
        return label
    date_text = registry.display_runtime_date(row.get("created_at", "") or row.get("updated_at", ""))
    if not date_text:
        return label
    detail = date_text
    suffix = row.get("label_suffix", "")
    if suffix:
        detail = f"{detail} {suffix}"
    return f"{label} ({detail})"


def _badges(row: dict[str, str], color_enabled: bool) -> str:
    badges: list[str] = []
    if row.get("kind") == "remote":
        badges.append(_badge("update", color_enabled))
    elif row.get("menu_latest") == "1":
        badges.append(_badge("latest", color_enabled))
    elif row.get("active") == "1":
        badges.append(_badge("active", color_enabled))
    else:
        badges.append(_badge("cached", color_enabled))
    return " ".join(badges)


def _badge(name: str, color_enabled: bool) -> str:
    palette = {
        "active": ("42;30", "🟢 active"),
        "cached": ("44;97", "📦 cached"),
        "install": ("43;30", "⬇ install"),
        "update": ("43;30", "⬇ update"),
        "latest": ("45;97", "⬆ latest"),
    }
    code, text = palette[name]
    return _color(code, f" {text} ", color_enabled)


def _number(index: str, color_enabled: bool) -> str:
    return _color("36", f"{index:>2}.", color_enabled)


def _dim(text: str, color_enabled: bool) -> str:
    return _color("2", text, color_enabled)


def _color(code: str, text: str, enabled: bool) -> str:
    if not enabled:
        return text
    return f"\033[{code}m{text}\033[0m"


def runtime_rows_from_registry(
    *,
    registry_file: Path,
    latest: str,
    runtime_store_dir: Path,
    runtime_builder: Path,
    patch_policy: str,
) -> list[dict[str, str]]:
    rows = registry.list_usable_runtimes(
        registry_file=registry_file,
        latest=latest,
        runtime_store_dir=runtime_store_dir,
        runtime_builder=runtime_builder,
        patch_policy=patch_policy,
    )
    _annotate_label_suffixes(rows)
    return rows


def _annotate_label_suffixes(rows: list[dict[str, str]]) -> None:
    counts: dict[tuple[str, str], int] = {}
    for row in rows:
        if row.get("kind") != "cached":
            continue
        key = (
            row.get("version", ""),
            registry.display_runtime_date(row.get("created_at", "") or row.get("updated_at", "")),
        )
        counts[key] = counts.get(key, 0) + 1
    for row in rows:
        row.pop("label_suffix", None)
        if row.get("kind") != "cached":
            continue
        key = (
            row.get("version", ""),
            registry.display_runtime_date(row.get("created_at", "") or row.get("updated_at", "")),
        )
        if counts.get(key, 0) <= 1:
            continue
        wrapper_version = row.get("wrapper_version", "")
        if wrapper_version:
            row["label_suffix"] = f"· {registry.display_wrapper_version(wrapper_version)}"
            continue
        wrapper_date = registry.display_runtime_date(row.get("wrapper_updated_at", ""))
        if wrapper_date:
            row["label_suffix"] = f"· {wrapper_date}"
            continue
        commit = row.get("wrapper_commit", "")[:7]
        if commit:
            row["label_suffix"] = f"· rev {commit}"
