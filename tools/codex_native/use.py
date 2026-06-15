"""Cached runtime listing and selection rendering."""

from __future__ import annotations

import os
import sys

from . import registry
from .errors import SchemaError


UNIT_SEPARATOR = "\x1f"


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


def selection_fields(row: dict[str, str]) -> str:
    return UNIT_SEPARATOR.join(
        [
            row["kind"],
            row.get("runtime_path", ""),
            row.get("raw_path", ""),
            row.get("version", "unknown"),
            row.get("raw_sha256", ""),
            row.get("runtime_sha256", ""),
            row.get("package_spec", ""),
        ]
    )


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
    print("Choose runtime", file=sys.stderr)
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
        print(_dim("  (More options: codex use <version>)", color_enabled), file=sys.stderr)
    print(file=sys.stderr)
    print(count)


def _menu_rows(rows: list[dict[str, str]]) -> tuple[dict[str, str] | None, list[dict[str, str]]]:
    latest_version = _latest_version(rows)
    latest = next(
        (
            row
            for row in rows
            if latest_version
            and row.get("active") != "1"
            and row.get("version") == latest_version
        ),
        None,
    )
    if latest is not None:
        latest["menu_latest"] = "1"
    for row in rows:
        if row is not latest:
            row.pop("menu_latest", None)
    remaining = [row for row in rows if row is not latest]
    return latest, remaining


def _latest_version(rows: list[dict[str, str]]) -> str:
    for row in rows:
        if row.get("kind") == "remote":
            return row.get("version", "")
    if not rows:
        return ""
    import re
    def version_key(v_str: str) -> tuple[tuple[int, int | str], ...]:
        v_clean = v_str
        for suffix in ("-linux-arm64", "-linux-x64", "-darwin-arm64", "-darwin-x64"):
            if v_clean.endswith(suffix):
                v_clean = v_clean[: -len(suffix)]
                break
        parts = re.split(r"[.-]", v_clean)
        key = []
        for p in parts:
            if not p:
                continue
            try:
                key.append((0, int(p)))
            except ValueError:
                key.append((1, p))
        return tuple(key)
    best_row = max(rows, key=lambda r: version_key(r.get("version", "")))
    return best_row.get("version", "")


def _menu_line(index: str, row: dict[str, str], color_enabled: bool) -> str:
    label = registry.display_version(row.get("version", "unknown"))
    parts = [f"  {_number(index, color_enabled)} {_label(label, row, color_enabled)}"]
    badges = _badges(row, color_enabled)
    if badges:
        parts.append(badges)
    return " ".join(parts)


def _label(label: str, row: dict[str, str], color_enabled: bool) -> str:
    return label


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


def runtime_rows_from_registry(**kwargs: object) -> list[dict[str, str]]:
    return registry.list_usable_runtimes(**kwargs)  # type: ignore[arg-type]
