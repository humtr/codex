#!/usr/bin/env python3
"""Emit the small main caller for a source commit whose gates have passed."""
import re
import sys


def caller(sha):
    if not re.fullmatch('[0-9a-f]{40}',sha):raise ValueError('expected immutable source SHA')
    header = """name: Termux Hosted Release Preflight
on:
  schedule:
    - cron: '0 */6 * * *'
  workflow_dispatch:
    inputs:
      publish:
        description: Publish after all real execution/readback gates
        type: boolean
        default: false
      rebuild_current:
        description: Rebuild the authenticated selected backend at the next sequence
        type: boolean
        default: false
      variant:
        description: Independent publication lane (both runs base then full)
        type: choice
        options: [both, base, full]
        default: both
permissions:
  contents: read
  actions: read
jobs:
  source:
    runs-on: ubuntu-24.04
    outputs:
      sha: ${{ steps.source.outputs.sha }}
    steps:
      - name: Resolve exact accepted product source
        id: source
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          sha="$(gh api repos/humtr/codex/contents/release-source-v1?ref=${{ github.sha }} --jq '.content' | base64 -d)"
          [[ "$sha" =~ ^[0-9a-f]{40}$ ]]
          for workflow in rald7-full-acceptance.yml core-runtime-qualification.yml; do
            gh api "repos/humtr/codex/actions/workflows/$workflow/runs?head_sha=$sha&per_page=100" > "$RUNNER_TEMP/runs.json"
            jq -e --arg sha "$sha" --arg path ".github/workflows/$workflow" \
              '.workflow_runs | any(.head_sha==$sha and .path==$path and .head_branch=="rewrite/rust-core" and (.event=="push" or .event=="workflow_dispatch") and .repository.full_name=="humtr/codex" and .status=="completed" and .conclusion=="success")' "$RUNNER_TEMP/runs.json"
          done
          printf 'sha=%s\\n' "$sha" >> "$GITHUB_OUTPUT"
"""
    jobs = []
    for variant, name in [('base', 'base'), ('full', 'release')]:
        dependency = "    needs: [source, base]\n" if variant == 'full' else "    needs: source\n"
        condition = ("${{ !cancelled() && (github.event_name == 'schedule' || inputs.variant != 'full') && needs.source.result == 'success' }}"
                     if variant == 'base' else
                     "${{ !cancelled() && (github.event_name == 'schedule' || inputs.variant != 'base') && needs.source.result == 'success' }}")
        jobs.append(f"""  {name}:
{dependency}    if: {condition}
    permissions:
      contents: write
      actions: read
      pages: write
      id-token: write
    uses: humtr/codex/.github/workflows/auto-release-termux.yml@{sha}
    with:
      source_sha: ${{{{ needs.source.outputs.sha }}}}
      release_variant: {variant}
      publish: ${{{{ github.event_name == 'schedule' || inputs.publish }}}}
      rebuild_current: ${{{{ github.event_name == 'workflow_dispatch' && inputs.rebuild_current }}}}
    secrets:
      CODEX_RELEASE_SIGNING_KEY: ${{{{ secrets.CODEX_RELEASE_SIGNING_KEY }}}}
""")
    return header + ''.join(jobs)


PROOF_WORKFLOWS = {
    'rald7-full-acceptance.yml': 'Source acceptance',
    'core-runtime-qualification.yml': 'Independent Core/runtime qualification',
    'manager-tui-native.yml': 'Manager TUI native qualification',
}


def dispatch_stubs():
    # GitHub requires dispatchable workflow names on its default branch. Runs
    # dispatched with ref=rewrite/rust-core execute that branch's real workflow,
    # whose head_sha is then checked by the controller. These discovery files
    # contain no product/release authority and reject accidental main execution.
    return {name: f"""name: {label}
on:
  workflow_dispatch:
{('    inputs:\n      rebuild_current:\n        type: boolean\n        default: true\n' if name.startswith('core-') else '')}permissions:
  contents: read
jobs:
  source_branch_required:
    runs-on: ubuntu-24.04
    steps:
      - run: |
          echo 'Dispatch this proof on rewrite/rust-core; main has no implementation.' >&2
          exit 1
""" for name, label in PROOF_WORKFLOWS.items()}



if __name__=='__main__':
    if len(sys.argv)!=2:raise SystemExit('usage: scripts/pin_release.py ACCEPTED_SOURCE_SHA')
    try:sys.stdout.write(caller(sys.argv[1]))
    except ValueError as e:raise SystemExit(str(e))
