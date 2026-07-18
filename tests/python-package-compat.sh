#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$(mktemp -d "$TMP_PARENT/codex-python-compat.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

checkout_file="$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B -c 'import codex_termux.cli as module; print(module.__file__)')"
[ "$checkout_file" = "$ROOT_DIR/src/wrapper/cli.py" ] || { printf 'python-package-compat: checkout resolved %s
' "$checkout_file" >&2; exit 1; }

src_file="$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B -c 'import codex_termux.cli as module; print(module.__file__)')"
[ "$src_file" = "$ROOT_DIR/src/wrapper/cli.py" ] || { printf 'python-package-compat: src resolved %s
' "$src_file" >&2; exit 1; }

mkdir -p "$TMP_DIR/manager/source/src"
cp -R "$ROOT_DIR/tools/codex_termux" "$TMP_DIR/manager/codex_termux"
cp -R "$ROOT_DIR/src/wrapper" "$TMP_DIR/manager/source/src/wrapper"
installed_file="$(PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$TMP_DIR/manager" python3 -B -c 'import codex_termux.cli as module; print(module.__file__)')"
[ "$installed_file" = "$TMP_DIR/manager/source/src/wrapper/cli.py" ] || { printf 'python-package-compat: installed resolved %s
' "$installed_file" >&2; exit 1; }

python3 -B - "$ROOT_DIR" <<'PY'
import ast
import sys
from pathlib import Path
root = Path(sys.argv[1])
for facade in (root / "tools/codex_termux", root / "src/codex_termux"):
    files = sorted(facade.glob("*.py"))
    assert [path.name for path in files] == ["__init__.py"], files
    tree = ast.parse(files[0].read_text(encoding="utf-8"))
    assert not any(isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)) for node in ast.walk(tree))
for path in sorted((root / "src/wrapper").glob("*.py")):
    compile(path.read_text(encoding="utf-8"), str(path), "exec")
PY
printf 'python-package-compat: ok
'
