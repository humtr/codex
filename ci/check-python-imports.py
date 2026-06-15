#!/usr/bin/env python3
"""Enforce codex_native import direction rules."""

from __future__ import annotations

import ast
import argparse
import sys
from pathlib import Path

PACKAGE = "codex_native"
ROOT = Path(__file__).resolve().parents[1]
PACKAGE_DIR = ROOT / "tools" / PACKAGE
DEFAULT_MAX_FUNCTION_LINES = 80

KNOWN_MODULES = {
    "errors",
    "paths",
    "schemas",
    "hashing",
    "atomic",
    "state",
    "registry",
    "store",
    "prune",
    "builder",
    "activation",
    "activation_cli",
    "migration",
    "doctor_report",
    "doctor_render",
    "runtime_checks",
    "use",
    "maintenance_cli",
    "cli",
}

ALLOWED_IMPORTS = {
    "__init__": set(),
    "errors": set(),
    "paths": {"errors", "hashing"},
    "schemas": {"errors"},
    "hashing": {"errors"},
    "atomic": {"errors"},
    "state": {"errors", "schemas", "atomic"},
    "registry": {"errors", "schemas", "hashing", "atomic", "paths"},
    "store": {"errors", "schemas", "hashing", "atomic", "registry", "prune"},
    "prune": {"errors", "schemas", "hashing", "atomic", "paths"},
    "builder": {"errors", "schemas", "hashing", "atomic"},
    "activation": {
        "errors",
        "schemas",
        "hashing",
        "atomic",
        "state",
        "registry",
        "store",
    },
    "activation_cli": {"schemas", "activation"},
    "migration": {"errors", "schemas", "hashing", "atomic", "registry", "store"},
    "use": {"errors", "schemas", "hashing", "paths", "registry"},
    "doctor_report": {
        "errors",
        "schemas",
        "hashing",
        "paths",
        "state",
        "registry",
        "store",
        "migration",
    },
    "doctor_render": {"errors", "schemas"},
    "runtime_checks": {"errors", "schemas", "hashing", "registry"},
    "maintenance_cli": {
        "errors",
        "schemas",
        "hashing",
        "paths",
        "prune",
        "migration",
        "doctor_report",
        "doctor_render",
        "runtime_checks",
        "use",
        "registry",
    },
    "cli": KNOWN_MODULES - {"cli"},
}


def _module_name(path: Path) -> str:
    return path.stem


def _top_level_module(name: str) -> str | None:
    if name == PACKAGE:
        return None
    if name.startswith(PACKAGE + "."):
        return name[len(PACKAGE) + 1 :].split(".", 1)[0]
    return None


def _relative_import_target(node: ast.ImportFrom) -> str | None:
    if node.level == 0:
        return _top_level_module(node.module or "")
    if node.level != 1:
        return "<unsupported-relative-import>"
    if node.module:
        return node.module.split(".", 1)[0]
    return None


def _imported_modules(tree: ast.AST) -> set[str]:
    imports: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                imported = _top_level_module(alias.name)
                if imported:
                    imports.add(imported)
        elif isinstance(node, ast.ImportFrom):
            imported = _relative_import_target(node)
            if imported:
                imports.add(imported)
            elif node.level == 0 and node.module == PACKAGE:
                for alias in node.names:
                    if alias.name in KNOWN_MODULES:
                        imports.add(alias.name)
            elif node.level == 1:
                for alias in node.names:
                    if alias.name in KNOWN_MODULES:
                        imports.add(alias.name)
    return imports


def _check_file(path: Path) -> list[str]:
    module = _module_name(path)
    if module not in ALLOWED_IMPORTS:
        return [f"{path}: unknown codex_native module '{module}'"]

    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except SyntaxError as exc:
        return [f"{path}: syntax error: {exc}"]

    failures: list[str] = []
    allowed = ALLOWED_IMPORTS[module]
    for imported in sorted(_imported_modules(tree)):
        if imported not in KNOWN_MODULES:
            failures.append(f"{path}: unknown intra-package import '{imported}'")
        elif imported not in allowed:
            failures.append(
                f"{path}: forbidden import '{imported}' from '{module}'"
            )
    return failures


def _check_python_function_lengths(
    paths: list[Path], max_lines: int
) -> list[str]:
    failures: list[str] = []
    for path in paths:
        try:
            tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        except SyntaxError as exc:
            failures.append(f"{path}: syntax error: {exc}")
            continue
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                end_lineno = getattr(node, "end_lineno", node.lineno)
                lines = end_lineno - node.lineno + 1
                if lines > max_lines:
                    failures.append(
                        f"{path}:{node.lineno}: function {node.name} has "
                        f"{lines} lines; limit is {max_lines}"
                    )
    return failures


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check codex_native import boundaries."
    )
    parser.add_argument(
        "--check-function-lengths",
        nargs="*",
        metavar="PATH",
        help="Also check Python function lengths for the supplied files.",
    )
    parser.add_argument(
        "--max-function-lines",
        type=int,
        default=DEFAULT_MAX_FUNCTION_LINES,
        help="Maximum Python function length for --check-function-lengths.",
    )
    return parser.parse_args(argv)


def main() -> int:
    args = _parse_args(sys.argv[1:])
    if not PACKAGE_DIR.is_dir():
        print(f"missing package directory: {PACKAGE_DIR}", file=sys.stderr)
        return 1

    failures: list[str] = []
    for path in sorted(PACKAGE_DIR.glob("*.py")):
        failures.extend(_check_file(path))
    if args.check_function_lengths is not None:
        length_paths = [Path(path) for path in args.check_function_lengths]
        failures.extend(
            _check_python_function_lengths(length_paths, args.max_function_lines)
        )

    if failures:
        print("codex_native Python checks failed:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
