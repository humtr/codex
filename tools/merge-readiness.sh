#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${CODEX_MERGE_READINESS_OUT_DIR:-$ROOT_DIR/out/merge-readiness}"
HANDOFF_DIR="$OUT_DIR/handoff"
TEST_LOG_DIR="$OUT_DIR/test-logs"
REPORT="$OUT_DIR/merge_readiness_report.json"
ROOT_REPORT="$ROOT_DIR/merge_readiness_report.json"
AUDIT_JSON="$OUT_DIR/canon-audit.json"
TEST_STATUS="$OUT_DIR/test-status.txt"
PATCH_FILE="$OUT_DIR/branch.patch"
REPO_ZIP="$OUT_DIR/repository-snapshot.zip"

mkdir -p "$HANDOFF_DIR" "$TEST_LOG_DIR"

repository="${GITHUB_REPOSITORY:-local/codex}"
head_sha="$(git rev-parse HEAD 2>/dev/null || printf unknown)"
base_sha="$(git rev-parse origin/main 2>/dev/null || git rev-parse main 2>/dev/null || printf unknown)"
branch="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || printf detached)"
created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
run_id="${GITHUB_RUN_ID:-local}"
run_attempt="${GITHUB_RUN_ATTEMPT:-local}"
run_url="${GITHUB_SERVER_URL:-https://github.com}/${repository}/actions/runs/${run_id}"
artifact_name="${CODEX_MERGE_READINESS_ARTIFACT_NAME:-codex-agent-handoff-${head_sha}}"

syntax_status=0
bash -n \
    install.sh \
    bin/install-local.sh \
    bin/install-runtime.sh \
    lib/codex-termux.sh \
    lib/codex-termux/*.sh \
    tests/*.sh \
    tools/merge-readiness.sh >"$OUT_DIR/syntax-check.log" 2>&1 || syntax_status=$?

validate_status=0
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools \
    python3 -B -m codex_termux.cli validate --root . >"$OUT_DIR/validate.log" 2>&1 || validate_status=$?

audit_status=0
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools \
    python3 -B -m codex_termux.cli canon-audit --root . --strict >"$AUDIT_JSON" 2>"$OUT_DIR/canon-audit.stderr" || audit_status=$?

audit_findings_count="$(python3 - "$AUDIT_JSON" <<'PY'
import json
import sys

try:
    print(len(json.load(open(sys.argv[1], encoding="utf-8")).get("findings", [])))
except Exception:
    print(-1)
PY
)"

test_exit_code=0
CODEX_TERMUX_TEST_LOG_DIR="$TEST_LOG_DIR" \
CODEX_TERMUX_TEST_STATUS_FILE="$TEST_STATUS" \
PYTHONDONTWRITEBYTECODE=1 \
    bash tests/run-all.sh >"$OUT_DIR/tests.stdout" 2>"$OUT_DIR/tests.stderr" || test_exit_code=$?

contract_status=ok
grep -q '^wrapper-contracts.sh 0$' "$TEST_STATUS" 2>/dev/null || contract_status=fail

test_status=ok
[ "$test_exit_code" -eq 0 ] || test_status=fail
audit_result=ok
[ "$audit_status" -eq 0 ] || audit_result=fail
validate_result=ok
[ "$validate_status" -eq 0 ] || validate_result=fail
syntax_result=ok
[ "$syntax_status" -eq 0 ] || syntax_result=fail
protected_path_status=ok
[ "$audit_result" = ok ] || protected_path_status=check-audit

main_delta=unknown
if git rev-parse origin/main >/dev/null 2>&1; then
    main_delta="$(git diff --shortstat origin/main...HEAD 2>/dev/null || printf unknown)"
fi

patch_status=0
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    if [ "$base_sha" != unknown ] && git rev-parse --verify "$base_sha^{commit}" >/dev/null 2>&1; then
        git diff --binary "${base_sha}"...HEAD >"$PATCH_FILE" 2>"$OUT_DIR/branch-patch.stderr" || patch_status=$?
    else
        git diff --binary >"$PATCH_FILE" 2>"$OUT_DIR/branch-patch.stderr" || patch_status=$?
    fi
else
    : >"$PATCH_FILE"
fi

snapshot_status=0
python3 - "$ROOT_DIR" "$REPO_ZIP" <<'PY' || snapshot_status=$?
import sys
import zipfile
from pathlib import Path

root = Path(sys.argv[1]).resolve()
out = Path(sys.argv[2]).resolve()
skip_dirs = {".git", "__pycache__", ".pytest_cache", ".mypy_cache", "out"}
skip_exact = {"merge_readiness_report.json"}
if out.exists():
    out.unlink()
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, allowZip64=True) as zf:
    for path in sorted(root.rglob("*")):
        if path.is_dir():
            continue
        rel_path = path.relative_to(root)
        rel = rel_path.as_posix()
        if rel in skip_exact:
            continue
        if any(part in skip_dirs for part in rel_path.parts):
            continue
        if path.resolve() == out:
            continue
        zf.write(path, rel)
PY

structural_score=0
[ -f codex-wrapper.manifest.json ] && structural_score=$((structural_score + 20))
[ -d lib/codex-termux ] && structural_score=$((structural_score + 20))
[ "$audit_result" = ok ] && structural_score=$((structural_score + 20))
[ "$test_status" = ok ] && structural_score=$((structural_score + 20))
[ "$contract_status" = ok ] && structural_score=$((structural_score + 20))

blockers=""
next_actions=""
add_blocker() { blockers="${blockers}${blockers:+;}$1"; }
add_next_action() { next_actions="${next_actions}${next_actions:+;}$1"; }

[ "$syntax_result" = ok ] || { add_blocker 'syntax checks failed'; add_next_action 'fix shell syntax errors'; }
[ "$validate_result" = ok ] || { add_blocker 'validate failed'; add_next_action 'inspect validate.log and restore required layout/contracts'; }
[ "$audit_result" = ok ] || { add_blocker 'canon-audit strict failed'; add_next_action 'inspect canon-audit.json and fix blockers'; }
[ "$test_status" = ok ] || { add_blocker 'tests failed'; add_next_action 'inspect test-logs and fix code/test gaps'; }
[ "$contract_status" = ok ] || { add_blocker 'compatibility contracts failed'; add_next_action 'fix wrapper-contracts.sh failures before refactoring further'; }
[ "$patch_status" -eq 0 ] || { add_blocker 'branch patch generation failed'; add_next_action 'inspect branch-patch.stderr and regenerate branch.patch'; }
[ "$snapshot_status" -eq 0 ] || { add_blocker 'repository snapshot generation failed'; add_next_action 'inspect repository-snapshot.zip generation and retry'; }

merge_readiness=not_ready
if [ -z "$blockers" ]; then
    merge_readiness=ready
    add_next_action 'ready for human review'
fi

python3 - \
    "$ROOT_DIR" \
    "$OUT_DIR" \
    "$HANDOFF_DIR" \
    "$REPORT" \
    "$ROOT_REPORT" \
    "$repository" \
    "$branch" \
    "$base_sha" \
    "$head_sha" \
    "$run_url" \
    "$run_id" \
    "$run_attempt" \
    "$artifact_name" \
    "$audit_result" \
    "$audit_findings_count" \
    "$test_status" \
    "$test_exit_code" \
    "$contract_status" \
    "$protected_path_status" \
    "$structural_score" \
    "$main_delta" \
    "$merge_readiness" \
    "$blockers" \
    "$next_actions" \
    "$created_at" \
    "$syntax_result" \
    "$validate_result" <<'PY'
import hashlib
import json
import os
import stat
import sys
from pathlib import Path

(
    root_dir,
    out_dir,
    handoff_dir,
    report_path,
    root_report_path,
    repository,
    branch,
    base_sha,
    head_sha,
    run_url,
    run_id,
    run_attempt,
    artifact_name,
    audit_status,
    audit_findings_count,
    test_status,
    test_exit_code,
    contract_status,
    protected_path_status,
    structural_score,
    main_delta,
    merge_readiness,
    blockers_raw,
    next_actions_raw,
    created_at,
    syntax_status,
    validate_status,
) = sys.argv[1:]

root = Path(root_dir)
out = Path(out_dir)
handoff = Path(handoff_dir)
report = Path(report_path)
root_report = Path(root_report_path)
try:
    out_rel = out.relative_to(root).as_posix()
except ValueError:
    out_rel = str(out)

blockers = [item for item in blockers_raw.split(";") if item]
next_actions = [item for item in next_actions_raw.split(";") if item]

artifact_paths = [
    f"{out_rel}/merge_readiness_report.json",
    f"{out_rel}/canon-audit.json",
    f"{out_rel}/canon-audit.stderr",
    f"{out_rel}/validate.log",
    f"{out_rel}/syntax-check.log",
    f"{out_rel}/tests.stdout",
    f"{out_rel}/tests.stderr",
    f"{out_rel}/test-status.txt",
    f"{out_rel}/test-logs/",
    f"{out_rel}/branch.patch",
    f"{out_rel}/repository-snapshot.zip",
    f"{out_rel}/handoff/connector-handoff.json",
    f"{out_rel}/handoff/NEXT_AGENT_PROMPT.md",
    f"{out_rel}/handoff/resume-from-artifact.sh",
    f"{out_rel}/handoff/artifact-manifest.json",
    "merge_readiness_report.json",
]

report_data = {
    "repository": repository,
    "branch": branch,
    "base_sha": base_sha,
    "head_sha": head_sha,
    "run_url": run_url,
    "run_id": run_id,
    "run_attempt": run_attempt,
    "artifact_name": artifact_name,
    "audit_status": audit_status,
    "audit_findings_count": int(audit_findings_count),
    "test_status": test_status,
    "test_exit_code": int(test_exit_code),
    "contract_status": contract_status,
    "protected_path_status": protected_path_status,
    "structural_score": int(structural_score),
    "main_delta": main_delta,
    "merge_readiness": merge_readiness,
    "blockers": blockers,
    "next_actions": next_actions,
    "artifact_paths": artifact_paths,
    "created_at": created_at,
    "syntax_status": syntax_status,
    "validate_status": validate_status,
}

resume_script = """#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARTIFACT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="${1:-$PWD/codex-resume}"
SNAPSHOT="$ARTIFACT_ROOT/repository-snapshot.zip"

[ -f "$SNAPSHOT" ] || { printf 'missing repository snapshot: %s\\n' "$SNAPSHOT" >&2; exit 2; }
[ ! -e "$TARGET_DIR" ] || { printf 'target already exists: %s\\n' "$TARGET_DIR" >&2; exit 2; }
mkdir -p "$TARGET_DIR"
python3 - "$SNAPSHOT" "$TARGET_DIR" <<'PYTHON'
import sys
import zipfile
from pathlib import Path

snapshot = Path(sys.argv[1])
target = Path(sys.argv[2])
with zipfile.ZipFile(snapshot) as zf:
    zf.extractall(target)
PYTHON
cd "$TARGET_DIR"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=tools \\
  python3 -B -m codex_termux.cli canon-audit --root . --strict
bash tests/run-all.sh
"""

next_prompt = f"""# Next Agent Prompt

Repository: {repository}
Branch: {branch}
Base SHA: {base_sha}
Head SHA: {head_sha}
Artifact name: {artifact_name}
Merge readiness: {merge_readiness}

Continue stabilization from this handoff artifact. Do not work on `main`, do not
merge the PR, do not force-push, and do not touch
`automation/canon-snapshot-index`.

Start by reading:

- `merge_readiness_report.json`
- `canon-audit.json`
- `test-status.txt`
- `handoff/connector-handoff.json`

If resuming from the artifact, run:

```sh
bash handoff/resume-from-artifact.sh ./codex-resume
```

Current blockers:

{chr(10).join(f"- {item}" for item in blockers) if blockers else "- none"}

Next actions:

{chr(10).join(f"- {item}" for item in next_actions) if next_actions else "- none"}
"""

connector_handoff = {
    "repository": repository,
    "branch": branch,
    "base_sha": base_sha,
    "head_sha": head_sha,
    "run_url": run_url,
    "run_id": run_id,
    "run_attempt": run_attempt,
    "artifact_name": artifact_name,
    "merge_readiness": merge_readiness,
    "blockers": blockers,
    "next_actions": next_actions,
    "artifact_paths": artifact_paths,
    "how_to_resume": [
        "download and unpack the GitHub artifact",
        "inspect merge_readiness_report.json",
        "run: bash handoff/resume-from-artifact.sh ./codex-resume",
    ],
}

handoff.mkdir(parents=True, exist_ok=True)
report.write_text(json.dumps(report_data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
root_report.write_text(json.dumps(report_data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
(handoff / "connector-handoff.json").write_text(
    json.dumps(connector_handoff, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
(handoff / "NEXT_AGENT_PROMPT.md").write_text(next_prompt, encoding="utf-8")
resume_path = handoff / "resume-from-artifact.sh"
resume_path.write_text(resume_script, encoding="utf-8")
resume_path.chmod(resume_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

manifest_entries = []
for path in sorted(out.rglob("*")):
    if path.is_dir():
        continue
    rel = path.relative_to(out).as_posix()
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest_entries.append({"path": rel, "size_bytes": path.stat().st_size, "sha256": digest})

(handoff / "artifact-manifest.json").write_text(
    json.dumps(
        {
            "artifact_name": artifact_name,
            "created_at": created_at,
            "files": manifest_entries,
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)

print(json.dumps(report_data, sort_keys=True))
PY

[ "$merge_readiness" = ready ]
