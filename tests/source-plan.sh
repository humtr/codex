#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'source-plan: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - <<'PYTHON' || fail 'source plan model failed'
from codex_termux import source


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

printf 'source-plan: ok\n'
