#!/usr/bin/env python3
"""Emit the small main caller for a source commit whose gates have passed."""
import re
import sys


def caller(sha):
    if not re.fullmatch('[0-9a-f]{40}',sha):raise ValueError('expected immutable source SHA')
    return '''name: Termux Hosted Release Preflight
on:
  schedule:
    - cron: '0 */6 * * *'
  workflow_dispatch:
    inputs:
      publish:
        description: Publish the qualified candidate after smoke; defaults to unsigned build only
        type: boolean
        default: false
      rebuild_current:
        description: Manually rebuild the authenticated qualified current backend at next sequence
        type: boolean
        default: false
permissions:
  contents: read
jobs:
  release:
    permissions:
      contents: write
      actions: read
      pages: write
      id-token: write
    uses: humtr/codex/.github/workflows/auto-release-termux.yml@'''+sha+'''
    with:
      source_sha: '''+sha+'''
      publish: ${{ github.event_name == 'schedule' || inputs.publish }}
      rebuild_current: ${{ github.event_name == 'workflow_dispatch' && inputs.rebuild_current }}
    secrets:
      CODEX_RELEASE_SIGNING_KEY: ${{ secrets.CODEX_RELEASE_SIGNING_KEY }}
'''


if __name__=='__main__':
    if len(sys.argv)!=2:raise SystemExit('usage: scripts/pin_release.py ACCEPTED_SOURCE_SHA')
    try:sys.stdout.write(caller(sys.argv[1]))
    except ValueError as e:raise SystemExit(str(e))
