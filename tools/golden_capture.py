#!/usr/bin/env python3
"""Capture and compare observable Codex Termux wrapper behavior.

The harness uses only the Python standard library so the same fixtures can run
in portable environments and on a Termux device.  It records command output,
exit status, selected environment values, filesystem changes, symlink targets,
and structured probe observations.
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Mapping


SCHEMA_VERSION = 1
TEXT_LIMIT = 64 * 1024
REQUIRED_LAYOUT_KEYS = {
    "source_layers",
    "target_paths",
    "compatibility_paths",
    "preserved_contracts",
    "migration_phases",
    "non_goals",
}


class GoldenError(RuntimeError):
    """Raised for invalid fixtures or capture failures."""


def load_json_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise GoldenError(f"failed to read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise GoldenError(f"invalid JSON in {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise GoldenError(f"{path} must contain a JSON object")
    return value


def canonical_json(value: Mapping[str, Any]) -> str:
    return json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"


def expand_string(value: str, context: Mapping[str, str]) -> str:
    try:
        return value.format_map(context)
    except KeyError as exc:
        raise GoldenError(f"unknown fixture placeholder {exc.args[0]!r} in {value!r}") from exc


def expand_list(values: Any, context: Mapping[str, str], field: str) -> list[str]:
    if not isinstance(values, list) or not all(isinstance(item, str) for item in values):
        raise GoldenError(f"case field {field} must be a list of strings")
    return [expand_string(item, context) for item in values]


def normalize_text(value: str, replacements: list[tuple[str, str]]) -> str:
    result = value
    for actual, marker in replacements:
        if actual:
            result = result.replace(actual, marker)
    return result


def path_replacements(context: Mapping[str, str]) -> list[tuple[str, str]]:
    pairs = [
        (context["root"], "<ROOT>"),
        (context["sandbox"], "<SANDBOX>"),
        (context["home"], "<HOME>"),
        (context["prefix"], "<PREFIX>"),
        (context["tmp"], "<TMP>"),
    ]
    return sorted(pairs, key=lambda item: len(item[0]), reverse=True)


def file_entry(path: Path, replacements: list[tuple[str, str]]) -> dict[str, Any]:
    if path.is_symlink():
        return {"type": "symlink", "target": normalize_text(os.readlink(path), replacements)}
    if not path.is_file():
        raise GoldenError(f"unsupported watched filesystem entry: {path}")
    try:
        payload = path.read_bytes()
    except OSError as exc:
        raise GoldenError(f"failed to read watched file {path}: {exc}") from exc
    entry: dict[str, Any] = {"type": "file"}
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError:
        text = ""
    if text and len(payload) <= TEXT_LIMIT:
        entry["text"] = normalize_text(text, replacements)
    elif not payload:
        entry["text"] = ""
    else:
        entry["sha256"] = hashlib.sha256(payload).hexdigest()
        entry["size"] = len(payload)
    return entry


def snapshot_filesystem(
    watches: list[tuple[str, Path]], replacements: list[tuple[str, str]]
) -> dict[str, dict[str, Any]]:
    snapshot: dict[str, dict[str, Any]] = {}
    for label, root in watches:
        if not root.exists() and not root.is_symlink():
            continue
        if root.is_file() or root.is_symlink():
            snapshot[label] = file_entry(root, replacements)
            continue
        for path in sorted(root.rglob("*")):
            if path.is_dir() and not path.is_symlink():
                continue
            relative = path.relative_to(root).as_posix()
            snapshot[f"{label}/{relative}"] = file_entry(path, replacements)
    return snapshot


def filesystem_diff(
    before: Mapping[str, dict[str, Any]], after: Mapping[str, dict[str, Any]]
) -> dict[str, dict[str, dict[str, Any]]]:
    created = {key: after[key] for key in sorted(after.keys() - before.keys())}
    deleted = {key: before[key] for key in sorted(before.keys() - after.keys())}
    modified = {
        key: {"before": before[key], "after": after[key]}
        for key in sorted(before.keys() & after.keys())
        if before[key] != after[key]
    }
    return {"created": created, "modified": modified, "deleted": deleted}


def parse_watches(case: Mapping[str, Any], context: Mapping[str, str]) -> list[tuple[str, Path]]:
    raw = case.get("watch", [])
    if not isinstance(raw, list):
        raise GoldenError("case field watch must be a list")
    watches: list[tuple[str, Path]] = []
    labels: set[str] = set()
    for item in raw:
        if not isinstance(item, dict):
            raise GoldenError("each watch entry must be an object")
        label = item.get("label")
        path = item.get("path")
        if not isinstance(label, str) or not label:
            raise GoldenError("watch label must be a non-empty string")
        if label in labels:
            raise GoldenError(f"duplicate watch label: {label}")
        if not isinstance(path, str) or not path:
            raise GoldenError(f"watch path for {label} must be a non-empty string")
        labels.add(label)
        watches.append((label, Path(expand_string(path, context))))
    return watches


def parse_observations(case: Mapping[str, Any], context: Mapping[str, str]) -> list[tuple[str, Path, str]]:
    raw = case.get("observations", [])
    if not isinstance(raw, list):
        raise GoldenError("case field observations must be a list")
    observations: list[tuple[str, Path, str]] = []
    labels: set[str] = set()
    for item in raw:
        if not isinstance(item, dict):
            raise GoldenError("each observation entry must be an object")
        label = item.get("label")
        path = item.get("path")
        kind = item.get("format", "text")
        if not isinstance(label, str) or not label:
            raise GoldenError("observation label must be a non-empty string")
        if label in labels:
            raise GoldenError(f"duplicate observation label: {label}")
        if not isinstance(path, str) or not path:
            raise GoldenError(f"observation path for {label} must be a non-empty string")
        if kind not in {"json", "text"}:
            raise GoldenError(f"unsupported observation format for {label}: {kind}")
        labels.add(label)
        observations.append((label, Path(expand_string(path, context)), kind))
    return observations


def read_observations(
    entries: list[tuple[str, Path, str]], replacements: list[tuple[str, str]]
) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for label, path, kind in entries:
        if not path.is_file():
            raise GoldenError(f"observation file was not produced: {path}")
        if kind == "json":
            value = load_json_object(path)
            encoded = canonical_json(value)
            result[label] = json.loads(normalize_text(encoded, replacements))
        else:
            result[label] = normalize_text(path.read_text(encoding="utf-8"), replacements)
    return result


def capture_case(case_path: Path) -> dict[str, Any]:
    case = load_json_object(case_path)
    if case.get("schema") != SCHEMA_VERSION:
        raise GoldenError(f"case schema must be {SCHEMA_VERSION}: {case_path}")
    name = case.get("name")
    if not isinstance(name, str) or not name:
        raise GoldenError(f"case name must be a non-empty string: {case_path}")

    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix=f"codex-golden-{name}.") as temp:
        sandbox = Path(temp).resolve()
        context = {
            "root": str(root),
            "sandbox": str(sandbox),
            "home": str(sandbox / "home"),
            "prefix": str(sandbox / "prefix"),
            "tmp": str(sandbox / "tmp"),
        }
        for key in ("home", "prefix", "tmp"):
            Path(context[key]).mkdir(parents=True, exist_ok=True)

        replacements = path_replacements(context)
        command = expand_list(case.get("command"), context, "command")
        if not command:
            raise GoldenError(f"case command must not be empty: {case_path}")

        env = dict(os.environ)
        env.update(
            {
                "HOME": context["home"],
                "PREFIX": context["prefix"],
                "TMPDIR": context["tmp"],
                "CODEX_TERMUX_HOME": context["home"],
                "CODEX_TERMUX_PREFIX": context["prefix"],
                "CODEX_TERMUX_STATE_DIR": str(sandbox / "state"),
                "CODEX_TERMUX_TMPDIR": context["tmp"],
                "CODEX_TERMUX_AUTO_UPDATE": "0",
                "LANG": "C",
                "LC_ALL": "C",
                "NO_COLOR": "1",
                "PYTHONDONTWRITEBYTECODE": "1",
                "PYTHONPATH": str(root / "src"),
                "TZ": "UTC",
            }
        )
        raw_env = case.get("environment", {})
        if not isinstance(raw_env, dict):
            raise GoldenError("case field environment must be an object")
        for key, value in raw_env.items():
            if not isinstance(key, str) or not isinstance(value, str):
                raise GoldenError("case environment keys and values must be strings")
            env[key] = expand_string(value, context)

        watches = parse_watches(case, context)
        observations = parse_observations(case, context)
        before = snapshot_filesystem(watches, replacements)
        timeout_seconds = case.get("timeout_seconds", 30)
        if not isinstance(timeout_seconds, int) or timeout_seconds <= 0:
            raise GoldenError("case timeout_seconds must be a positive integer")
        try:
            completed = subprocess.run(
                command,
                cwd=root,
                env=env,
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=timeout_seconds,
            )
        except subprocess.TimeoutExpired as exc:
            raise GoldenError(f"case timed out after {timeout_seconds}s: {name}") from exc
        after = snapshot_filesystem(watches, replacements)

        capture_names = case.get("capture_environment", [])
        if not isinstance(capture_names, list) or not all(isinstance(item, str) for item in capture_names):
            raise GoldenError("case field capture_environment must be a list of strings")
        captured_env = {
            key: normalize_text(env.get(key, ""), replacements) for key in sorted(capture_names)
        }

        return {
            "schema": SCHEMA_VERSION,
            "name": name,
            "command": [normalize_text(item, replacements) for item in command],
            "exit_code": completed.returncode,
            "stdout": normalize_text(completed.stdout.decode("utf-8", "replace"), replacements),
            "stderr": normalize_text(completed.stderr.decode("utf-8", "replace"), replacements),
            "environment": captured_env,
            "filesystem": filesystem_diff(before, after),
            "observations": read_observations(observations, replacements),
        }


def expected_path_for(case_path: Path, name: str) -> Path:
    golden_root = case_path.parent.parent
    return golden_root / "expected" / f"{name}.json"


def compare_case(case_path: Path, *, update: bool) -> int:
    actual = capture_case(case_path)
    expected_path = expected_path_for(case_path, str(actual["name"]))
    actual_text = canonical_json(actual)
    if update:
        expected_path.parent.mkdir(parents=True, exist_ok=True)
        expected_path.write_text(actual_text, encoding="utf-8")
        print(f"updated {expected_path}")
        return 0
    expected = load_json_object(expected_path)
    expected_text = canonical_json(expected)
    if actual_text == expected_text:
        print(f"golden: {actual['name']}: ok")
        return 0
    diff = difflib.unified_diff(
        expected_text.splitlines(keepends=True),
        actual_text.splitlines(keepends=True),
        fromfile=str(expected_path),
        tofile=f"actual:{actual['name']}",
    )
    sys.stderr.writelines(diff)
    return 1


def validate_layout_contract(path: Path) -> int:
    contract = load_json_object(path)
    if contract.get("schema") != SCHEMA_VERSION:
        raise GoldenError(f"layout contract schema must be {SCHEMA_VERSION}")
    missing = sorted(REQUIRED_LAYOUT_KEYS - set(contract))
    if missing:
        raise GoldenError("layout contract is missing fields: " + ", ".join(missing))
    if contract.get("baseline_commit") != "803fef15fe17960bdf8e9f7d5d199b813d08224b":
        raise GoldenError("layout contract baseline commit is not 803fef1")
    preserved = contract.get("preserved_contracts")
    if not isinstance(preserved, list) or not preserved:
        raise GoldenError("layout contract preserved_contracts must be a non-empty list")
    print(f"golden layout contract: {path}: ok")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--case", type=Path, help="golden case JSON to execute")
    group.add_argument("--validate-layout", type=Path, help="role layout contract JSON to validate")
    parser.add_argument("--update", action="store_true", help="replace the expected result")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.validate_layout is not None:
            if args.update:
                raise GoldenError("--update is only valid with --case")
            return validate_layout_contract(args.validate_layout)
        return compare_case(args.case, update=args.update)
    except GoldenError as exc:
        print(f"golden: ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
