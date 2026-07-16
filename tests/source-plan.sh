#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_PARENT="${TMPDIR:-${RUNNER_TEMP:-/tmp}}"
TMP_DIR="$(mktemp -d "$TMP_PARENT/codex-source-plan.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
    printf 'source-plan: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - <<'PYTHON' \
    || fail 'source plan model failed'
import tempfile
from pathlib import Path

from wrapper import source


def write_wrapper_layout(root: Path) -> None:
    for relative in source.REQUIRED_WRAPPER_SOURCE_PATHS:
        path = root / relative
        if relative == "tools/codex_termux":
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

plan = source.wrapper_source_plan(release_url="/tmp/wrapper.tar.gz")
assert plan.kind == "release", plan
assert plan.release_url == "/tmp/wrapper.tar.gz", plan

plan = source.wrapper_source_plan(local_root="/repo")
assert plan.kind == "local", plan
assert plan.local_root == "/repo", plan
assert plan.label == "local /repo", plan
assert "CODEX_WRAPPER_SOURCE_KIND=local" in source.wrapper_source_plan_exports(plan)

env = source.normalized_source_env(
    {
        "CODEX_TERMUX_WRAPPER_GIT_REPO": "legacy/private",
        "CODEX_TERMUX_WRAPPER_GIT_REF": "dev",
        "CODEX_TERMUX_WRAPPER_GIT_TOKEN": "legacy-token",
    }
)
assert env == {
    "CODEX_TERMUX_WRAPPER_REPO": "legacy/private",
    "CODEX_TERMUX_WRAPPER_REF": "dev",
    "CODEX_TERMUX_WRAPPER_TOKEN": "legacy-token",
}, env
assert source.source_env_exports(
    {"CODEX_TERMUX_WRAPPER_REPO": "owner/repo with space"}
) == "export CODEX_TERMUX_WRAPPER_REPO='owner/repo with space'"
assert source.auth_token({"CODEX_TERMUX_WRAPPER_TOKEN": "direct", "GITHUB_TOKEN": "github"}) == "direct"
assert source.auth_token({"CODEX_TERMUX_WRAPPER_GIT_TOKEN": "legacy", "GITHUB_TOKEN": "github"}) == "legacy"
assert source.auth_token({"GITHUB_TOKEN": "github"}) == "github"
assert source.auth_token({}) == ""
assert source.source_commit(Path("/definitely/not/a/git/repo")) == "unknown"

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
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli wrapper-source-plan \
            --repo humtr/codex --ref main --local-root "$ROOT_DIR" --field kind
)"
[ "$kind" = "git" ] || fail "CLI kind field mismatch: $kind"

url="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli wrapper-source-plan \
            --repo humtr/codex --ref main --local-root "$ROOT_DIR" --field git-url
)"
[ "$url" = "https://github.com/humtr/codex.git" ] || fail "CLI git-url mismatch: $url"

plan_env="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli wrapper-source-plan-env \
            --repo humtr/codex --ref main --local-root "$ROOT_DIR"
)"
case "$plan_env" in
    *"CODEX_WRAPPER_SOURCE_KIND=git"*\
*"CODEX_WRAPPER_SOURCE_GIT_URL=https://github.com/humtr/codex.git"*\
*"CODEX_WRAPPER_SOURCE_LABEL=github.com/humtr/codex@main"*) ;;
    *) fail "CLI source plan env mismatch: $plan_env" ;;
esac

extract_root="$TMP_DIR/extract"
mkdir -p "$extract_root/codex-release"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" python3 -B - \
    "$extract_root/codex-release" <<'PYTHON'
import sys
from pathlib import Path
from wrapper import source
root = Path(sys.argv[1])
for relative in source.REQUIRED_WRAPPER_SOURCE_PATHS:
    path = root / relative
    if relative == "tools/codex_termux":
        path.mkdir(parents=True, exist_ok=True)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("test\n", encoding="utf-8")
PYTHON

resolved="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli wrapper-source-root --extract-root "$extract_root"
)"
expected="$(cd "$extract_root/codex-release" && pwd)"
[ "$resolved" = "$expected" ] || fail "CLI wrapper-source-root mismatch: $resolved"

auth_token="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli wrapper-auth-token \
            --git-token legacy-token --github-token github-token
)"
[ "$auth_token" = "legacy-token" ] || fail "legacy CLI auth-token mismatch: $auth_token"

mkdir -p "$TMP_DIR/bin"
cat >"$TMP_DIR/bin/gh" <<'SCRIPT'
#!/bin/sh
[ "$1" = "auth" ] && [ "$2" = "token" ] || exit 2
printf '%s\n' fake-gh-token
SCRIPT
chmod 755 "$TMP_DIR/bin/gh"
auth_token="$(
    PATH="$TMP_DIR/bin:$PATH" PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli wrapper-auth-token --allow-gh 1
)"
[ "$auth_token" = "fake-gh-token" ] || fail "CLI wrapper-auth-token gh fallback mismatch: $auth_token"

commit="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/src" \
        python3 -B -m wrapper.cli wrapper-source-commit --root "$extract_root/codex-release"
)"
[ "$commit" = "unknown" ] || fail "CLI wrapper-source-commit non-git mismatch: $commit"

printf 'source-plan: ok\n'
