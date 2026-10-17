"""One completion relay; it has no qualification or publication authority."""
import re


def signal_job(dependency, legacy=False):
    if dependency not in {'accepted', 'qualify', 'upstream-baseline'}:
        raise ValueError('unknown qualification dependency')
    condition=f"always() && needs.{dependency}.result == 'success'" if legacy else 'always()'
    return f'''
  notify_acceptance:
    needs: {dependency}
    if: {condition}
    permissions:
      contents: read
      actions: write
    runs-on: ubuntu-24.04
    steps:
      - name: Continue accepted input processing
        env:
          GH_TOKEN: ${{{{ github.token }}}}
        run: |
          set -euo pipefail
          if gh api 'repos/humtr/codex/contents/.github/workflows/accepted-release-inputs.yml?ref=main' --silent; then
            gh workflow run accepted-release-inputs.yml --repo humtr/codex --ref main -f completed_run="$GITHUB_RUN_ID"
          fi
'''


def native_recipe(workflow):
    # Only this exact, isolated trailing notification job is outside the build
    # recipe. Global settings and every build/test step remain byte-bound.
    for legacy in (False, True):
        tail = signal_job('upstream-baseline', legacy=legacy)
        if workflow.endswith(tail):
            workflow = workflow[:-len(tail)]
            break
    # Subscription syntax is metadata only when no build/test command consumes
    # event/dispatch input. Future input-dependent recipes retain the whole file.
    match = re.search(r'(?m)^on:\n(?:[ \t].*\n|\n)*?(?=^permissions:)', workflow)
    if match:
        rest = workflow[:match.start()] + workflow[match.end():]
        if not any(token in rest for token in ['inputs.', 'github.event', 'GITHUB_EVENT']):
            return rest
    return workflow
