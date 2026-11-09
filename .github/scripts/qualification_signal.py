"""Shared exact-run admission, source recipes and completion signals."""
import json
import re
import subprocess

REPO = 'humtr/codex'
SOURCE_BRANCH = 'rewrite/rust-core'


class GitHubFailure(subprocess.CalledProcessError):
    def __init__(self, code, operation, detail):
        super().__init__(code,['gh',operation])
        self.operation,self.detail=operation,detail

    def __str__(self):
        return 'GitHub '+self.operation+' operation failed: '+self.detail


def gh(*args, payload=None):
    command = ['gh', *args]
    if payload is not None:
        command += ['--input', '-']
    try:
        result = subprocess.run(command, input=None if payload is None else json.dumps(payload).encode(),
                                capture_output=True, check=True)
    except subprocess.CalledProcessError as error:
        status=re.search(r'HTTP ([0-9]{3})',error.stderr.decode(errors='replace'))
        detail='HTTP '+status[1] if status else 'exit '+str(error.returncode)
        # Keep bearer tokens, signed transport URLs and API response bodies out
        # of logs while preserving an actionable operation/status boundary.
        raise GitHubFailure(error.returncode,args[0],detail) from None
    return result.stdout


def api(path, *, payload=None, method=None):
    args = ['api', 'repos/'+REPO+'/'+path]
    if method:args += ['--method', method]
    value = gh(*args, payload=payload)
    return json.loads(value) if value else None


def owned_run(run, workflow, sha):
    if workflow == 'manager-tui-native.yml':
        title = run.get('display_title')
        if not isinstance(title, str) or title.startswith('Focused Native '):
            return False
    expected={'path':'.github/workflows/'+workflow,'head_sha':sha,'head_branch':SOURCE_BRANCH}
    return (all(run.get(k)==value for k,value in expected.items())
            and run.get('event') in ['push','workflow_dispatch']
            and run.get('repository',{}).get('full_name')==REPO)


def accepted_run(run, workflow, sha):
    return (owned_run(run,workflow,sha) and run.get('status')=='completed'
            and run.get('conclusion')=='success')


def successful_run(workflow, sha):
    runs = api('actions/workflows/'+workflow+'/runs?head_sha='+sha+'&per_page=100')['workflow_runs']
    return next((run for run in runs if accepted_run(run, workflow, sha)), None), runs


def qualified_runtime_run(source):
    """A completed Core qualification, independent of branch-local cache access."""
    run, _ = successful_run('core-runtime-qualification.yml', source)
    if run is None:
        return None
    page = api('actions/runs/'+str(run['id'])+'/artifacts?per_page=100')
    artifacts = page['artifacts']
    if page['total_count'] != len(artifacts):
        raise ValueError('incomplete qualified runtime artifact listing')
    matches = [artifact for artifact in artifacts
               if artifact['name'] == 'termux-runtime-source' and artifact['expired'] is False]
    if len(matches) > 1:
        raise ValueError('ambiguous qualified runtime artifact')
    return run['id'] if matches else None


def signal_job(dependency, legacy=False):
    if dependency not in {'accepted', 'qualify', 'upstream-baseline'}:
        raise ValueError('unknown qualification dependency')
    condition=f"always() && needs.{dependency}.result == 'success'" if legacy else 'always()'
    if dependency == 'qualify':
        condition += " && github.ref == 'refs/heads/rewrite/rust-core'"
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
    # Isolated notification and scheduling metadata do not change built bytes.
    # Build settings and every build/test step remain byte-bound.
    for legacy in (False, True):
        tail = signal_job('upstream-baseline', legacy=legacy)
        if workflow.endswith(tail):
            workflow = workflow[:-len(tail)]
            break
    job = re.search(r'(?m)^  upstream-baseline:\n', workflow)
    if job:
        rest = workflow[job.end():]
        next_job = re.search(r'(?m)^  [A-Za-z0-9_-]+:\s*\n', rest)
        end = job.end() + next_job.start() if next_job else len(workflow)
        body = workflow[job.end():end]
        # Only the literal budget of this known job, including its YAML comments.
        # Expressions or command references to that field stay byte-bound.
        if body.count('timeout-minutes') == 1:
            body = re.sub(r'(?m)^(?:    #[^\n]*\n)*    timeout-minutes: [1-9][0-9]*\n', '', body)
            workflow = workflow[:job.end()] + body + workflow[end:]
    # Subscription syntax is metadata only when no build/test command consumes
    # event/dispatch input. Future input-dependent recipes retain the whole file.
    match = re.search(r'(?m)^on:\n(?:[ \t].*\n|\n)*?(?=^permissions:)', workflow)
    if match:
        rest = workflow[:match.start()] + workflow[match.end():]
        if not any(token in rest for token in ['inputs.', 'github.event', 'GITHUB_EVENT']):
            return rest
    return workflow
