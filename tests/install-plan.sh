#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    printf 'install-plan: FAIL: %s\n' "$*" >&2
    exit 1
}

PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" python3 -B - <<'PYTHON' || fail 'install plan model failed'
from codex_termux import install_plan


def check(command, args, *, action, surface="", version="", exit_code=0, error=""):
    result = install_plan.plan(command, args)
    assert result.action == action, result
    assert result.surface == surface, result
    assert result.version == version, result
    assert result.exit_code == exit_code, result
    assert result.error == error, result


check("install", [], action=install_plan.ACTION_INSTALL_FULL, surface="install")
assert install_plan.plan("install", []).to_dict()["surface_message"] == "Installing wrapper support and fresh upstream runtime"
check("install", ["0.142.0"], action=install_plan.ACTION_INSTALL_FULL, surface="install", version="0.142.0")
check("install", ["support"], action=install_plan.ACTION_SUPPORT, surface="support")
assert install_plan.plan("install", ["support"]).to_dict()["success_message"] == "Support files and launcher are ready"
exports = install_plan.plan_shell_exports("install", ["upstream", "0.142.0"])
assert "CODEX_INSTALL_PLAN_ACTION=upstream" in exports
assert "CODEX_INSTALL_PLAN_VERSION=0.142.0" in exports
assert "CODEX_INSTALL_PLAN_SURFACE_MESSAGE='Installing upstream Codex 0.142.0'" in exports
check("install", ["upstream"], action=install_plan.ACTION_UPSTREAM, surface="upstream")
assert install_plan.plan("install", ["upstream"]).to_dict()["surface_message"] == "Installing latest upstream Codex"
check("install", ["upstream", "0.142.0"], action=install_plan.ACTION_UPSTREAM, surface="upstream", version="0.142.0")
assert install_plan.plan("install", ["upstream", "0.142.0"]).to_dict()["surface_message"] == "Installing upstream Codex 0.142.0"
check("install", ["rebuild"], action=install_plan.ACTION_REBUILD, surface="rebuild")
check("update", [], action=install_plan.ACTION_INSTALL_FULL, surface="update")
check("update", ["0.142.0"], action=install_plan.ACTION_INSTALL_FULL, surface="update", version="0.142.0")
check("repair", [], action=install_plan.ACTION_REPAIR, surface="repair")
check("install", ["--help"], action=install_plan.ACTION_USAGE)
check("install", ["support", "--help"], action=install_plan.ACTION_USAGE)
check("update", ["help"], action=install_plan.ACTION_USAGE)
check(
    "install",
    ["upstream", "--bad-option"],
    action=install_plan.ACTION_ERROR,
    exit_code=64,
    error="install upstream version must not start with '-': --bad-option",
)
check(
    "install",
    ["upstream", "0.1.0", "extra"],
    action=install_plan.ACTION_ERROR,
    exit_code=64,
    error="install upstream takes at most one version argument",
)
check(
    "install",
    ["support", "extra"],
    action=install_plan.ACTION_ERROR,
    exit_code=2,
    error="install support does not take arguments",
)
check(
    "repair",
    ["extra"],
    action=install_plan.ACTION_ERROR,
    exit_code=2,
    error="repair does not take arguments",
)
PYTHON

action="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli install-plan --command install --field action -- upstream 0.142.0
)"
[ "$action" = "upstream" ] || fail "CLI action field mismatch: $action"

version="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli install-plan --command install --field version -- upstream 0.142.0
)"
[ "$version" = "0.142.0" ] || fail "CLI version field mismatch: $version"

message="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli install-plan --command install --field surface-message -- upstream 0.142.0
)"
[ "$message" = "Installing upstream Codex 0.142.0" ] || fail "CLI message field mismatch: $message"

env_plan="$(
    PYTHONDONTWRITEBYTECODE=1 PYTHONPATH="$ROOT_DIR/tools" \
        python3 -B -m codex_termux.cli install-plan-env --command install -- upstream 0.142.0
)"
printf '%s\n' "$env_plan" | grep -Fx "CODEX_INSTALL_PLAN_ACTION=upstream" >/dev/null \
    || fail "CLI install-plan-env action mismatch: $env_plan"
printf '%s\n' "$env_plan" | grep -Fx "CODEX_INSTALL_PLAN_VERSION=0.142.0" >/dev/null \
    || fail "CLI install-plan-env version mismatch: $env_plan"

printf 'install-plan: ok\n'
