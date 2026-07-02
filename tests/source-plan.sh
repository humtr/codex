#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"

fail() {
    printf 'source-plan: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - <<'PYTHON' || fail 'source plan model failed'
import tempfile
from pathlib import Path

from codex_termux import source


def write_wrapper_layout(root: Path) -> None:
    for relative in source.REQUIRED_WRAPPER_SOURCE_PATHS:
        path = root / relative
        if relative.endswith("codex_termux") or relative.endswith("lib/codex-termux"):
            path.mkdir(parents=True, exist_ok=True)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("test\n", encoding="utf-8")


plan = source.wrapper_source_plan(repo="humtr/codex", ref="main")
assert plan.kind == "git", plan
assert plan.git_url == "https://github.com/humtr/codex.git", plan
assert plan.label == "github.com/humtr/codex@main", plan

plan = source.wrapper_source_plan(repo="ssh://example.test/repo.git", ref="dev")
assert plan.kind == "git", plan
assert plan.git_url == "ssh://example.test/repo.git", plan
assert plan.label == "ssh://example.test/repo.git@dev", plan

plan = source.wrapper_source_plan(release_repo="humtr/codex", release_tag="v1")
assert plan.kind == "release", plan
assert plan.release_url == "https://github.com/humtr/codex/archive/refs/tags/v1.tar.gz", plan
assert plan.label == "release archive", plan

plan = source.wrapper_source_plan(release_url="/tmp/wrapper.tar.gz")
assert plan.kind == "release", plan
assert plan.release_url == "/tmp/wrapper.tar.gz", plan

plan = source.wrapper_source_plan(local_root="/repo")
assert plan.kind == "local", plan
assert plan.local_root == "/repo", plan
assert plan.label == "local /repo", plan

with tempfile.TemporaryDirectory() as tmp:
    root = Path(tmp)
    write_wrapper_layout(root)
    assert source.find_extracted_wrapper_source(root) == root.resolve()

with tempfile.TemporaryDirectory() as tmp:
    root = Path(tmp)
    nested = root / "codex-release"
    write_wrapper_layout(nested)
    assert source.find_extracted_wrapper_source(root) == nested.resolve()

with tempfile.TemporaryDirectory() as tmp:
    try:
        source.find_extracted_wrapper_source(Path(tmp))
    except Exception as exc:
        assert "wrapper source root not found" in str(exc), exc
    else:
        raise AssertionError("empty extracted archive unexpectedly resolved")
PYTHON

kind="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli wrapper-source-plan \
            --repo humtr/codex --ref main --local-root "$ROOT_DIR" --field kind
)"
[ "$kind" = "git" ] || fail "CLI kind field mismatch: $kind"

url="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli wrapper-source-plan \
            --repo humtr/codex --ref main --local-root "$ROOT_DIR" --field git-url
)"
[ "$url" = "https://github.com/humtr/codex.git" ] || fail "CLI git-url mismatch: $url"

extract_root="$TMP_PARENT/codex-source-plan-extract.$$"
trap 'rm -rf "$extract_root"' EXIT
mkdir -p "$extract_root/codex-release/bin" "$extract_root/codex-release/lib/codex-termux" \
    "$extract_root/codex-release/tools/codex_termux" "$extract_root/codex-release/config"
printf 'test\n' >"$extract_root/codex-release/install.sh"
printf 'test\n' >"$extract_root/codex-release/bin/install-local.sh"
printf 'test\n' >"$extract_root/codex-release/bin/install-runtime.sh"
printf 'test\n' >"$extract_root/codex-release/lib/codex-termux.sh"
for domain in dispatch state profile session runtime notify doctor; do
    printf 'test\n' >"$extract_root/codex-release/lib/codex-termux/$domain.sh"
done
printf 'test\n' >"$extract_root/codex-release/codex-wrapper.manifest.json"
printf 'test\n' >"$extract_root/codex-release/tools/build-runtime.py"
printf 'test\n' >"$extract_root/codex-release/tools/bwrap-termux-compat.py"
printf 'test\n' >"$extract_root/codex-release/tools/rg-termux-shim.sh"
printf 'test\n' >"$extract_root/codex-release/tools/codex-turn-notify.sh"
printf 'test\n' >"$extract_root/codex-release/tools/codex-launcher.c"
printf 'test\n' >"$extract_root/codex-release/config/wrapper-version.env"
resolved="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli wrapper-source-root --extract-root "$extract_root"
)"
expected="$(cd "$extract_root/codex-release" && pwd)"
[ "$resolved" = "$expected" ] || fail "CLI wrapper-source-root mismatch: $resolved"

printf 'source-plan: ok\n'
