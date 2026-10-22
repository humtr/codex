#!/usr/bin/env python3
"""Android product composition/relocation proof in disposable signed fixtures.

Runs the real Core and Provider, with no Chat inference or existing app profile.
Fixture signatures do not qualify a public Provider package or device activation.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

from qualify_permission_lifecycle import fixture
from clean_development import fixture_cleanup


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def qualify(args):
    root = Path(tempfile.mkdtemp(prefix='g-product-', dir=os.environ['TMPDIR']))
    report = dict(schema='termux-codex-provider-product-proof-v1', accepted=False,
                  root=str(root), started_UTC=utc(), cases=[], actual_Chat_requests=0,
                  native_development_turns=0, public_package_qualified=False,
                  operating_runtime_changed=False)
    started = time.monotonic()
    inputs = {name: sha(getattr(args, name))
              for name in ('core', 'frontend', 'provider', 'previous_provider')}
    inputs['generation_manifest'] = sha(args.generation / 'release.manifest')
    inputs['driver'] = sha(Path(__file__).resolve())
    inputs['fixture_driver'] = sha(Path(__file__).with_name('qualify_permission_lifecycle.py'))
    source = Path(__file__).resolve().parents[1]
    helpers = ('scripts/qualify_provider_product.py', 'scripts/qualify_permission_lifecycle.py',
               'scripts/clean_development.py',
               'scripts/qualify_tasks.py', 'scripts/fixtures/permission-model.json',
               '.github/scripts/rald3_preflight.py', '.github/scripts/rald5_publication.py')
    inputs['helpers'] = {}
    for name in helpers:
        copied = root / 'source' / name
        copied.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / name, copied)
        inputs['helpers'][name] = sha(copied)
    report['inputs'] = inputs
    (root / 'inputs.json').write_text(json.dumps(inputs, indent=2) + '\n')
    try:
        for composition in ('base', 'manager-no-tui', 'full'):
            case_root = root / composition
            case_root.mkdir(mode=0o700)
            case = dict(composition=composition, accepted=False, steps=[])
            report['cases'].append(case)
            home, prefix, env = fixture(case_root, args.core, args.frontend,
                                        generation=args.generation)
            env.update(XDG_CONFIG_HOME=str(case_root / 'config'),
                       XDG_STATE_HOME=str(case_root / 'state'))
            launcher = prefix / 'bin/codex'
            state = case_root / 'provider-state'
            previous = case_root / 'payloads/previous/codex-chatgpt-provider'
            current = case_root / 'payloads/current/codex-chatgpt-provider'
            for destination, input_binary in ((previous, args.previous_provider),
                                              (current, args.provider)):
                destination.parent.mkdir(parents=True, mode=0o700)
                shutil.copy2(input_binary, destination)
                destination.chmod(0o700)
            active = None

            def run(label, command, expected=0, secret=False, selected=None):
                begin = time.monotonic()
                entry = dict(name=label, started_UTC=utc())
                case['steps'].append(entry)
                result = subprocess.run([str(v) for v in command], env=selected or env,
                                        cwd=case_root / 'w', capture_output=True, timeout=65)
                entry.update(seconds=time.monotonic() - begin, exit=result.returncode,
                             expected_exit=expected, ended_UTC=utc())
                if not secret:
                    log = case_root / f'{len(case["steps"]):02d}.stdout'
                    log.write_bytes(result.stdout)
                    entry['stdout_sha256'] = sha(log)
                    (log.with_suffix('.stderr')).write_bytes(result.stderr)
                assert result.returncode == expected, (
                    label, result.returncode, result.stderr[-1024:].decode(errors='replace'))
                return result

            # Prune only our generated copy, then sign its exact smaller inventory.
            installed = home / '.local/lib/codex/core/generations/local-permission-fixture'
            if composition != 'full':
                removed = {'helpers/2'}
                if composition == 'base':
                    removed.add('manager')
                for name in removed:
                    (installed / name).unlink()
                descriptor = (installed / 'generation.meta').read_text().splitlines()
                descriptor = [line for line in descriptor
                              if not line.startswith('helper\ttermux-manager-tui-v1:')]
                descriptor = ['helper_count\t2' if line.startswith('helper_count\t') else
                              'manager_artifact_digest\t-' if composition == 'base'
                              and line.startswith('manager_artifact_digest\t') else line
                              for line in descriptor]
                (installed / 'generation.meta').write_text('\n'.join(descriptor) + '\n')
                manifest = (installed / 'release.manifest').read_text().splitlines()
                manifest = [line for line in manifest
                            if not (line.startswith('file\t') and line.split('\t')[1] in removed)]
                manifest = ['file_count\t' + str(8 - len(removed))
                            if line.startswith('file_count\t') else
                            '\t'.join(['file', 'generation.meta', sha(installed / 'generation.meta'), '0644'])
                            if line.startswith('file\tgeneration.meta\t') else line
                            for line in manifest]
                (installed / 'release.manifest').write_text('\n'.join(manifest) + '\n')
                run('sign isolated composition', ['openssl', 'pkeyutl', '-sign', '-inkey',
                    case_root / 'fixture-key.pem', '-rawin', '-in', installed / 'release.manifest',
                    '-out', installed / 'release.sig'])
            sentinels = {p: sha(p) for p in (home / '.codex/config.toml', home / '.codex/auth.json',
                                            home / '.local/share/codex/core/activation-state')}
            endpoint = original_credential = None
            retained_catalogs = {}
            try:
                run('native version without Provider registration', [launcher, '--version'])
                run('native config load without Provider', [launcher, 'features', 'list'])
                if composition == 'base':
                    missing = run('Manager absence', [launcher, 'termux', '--help'], expected=1)
                    assert b'Manager is unavailable' in missing.stderr
                run('configure explicit isolated native binding',
                    [previous, 'config', 'set', 'execution.core', launcher,
                     'execution.home', home / '.codex', 'execution.workspace', case_root / 'w'])
                settings = case_root / 'config/codex/providers/chatgpt.toml'
                settings_hash = sha(settings)
                for index, binary in enumerate((previous, current, current, previous)):
                    label = ('previous install', 'current relocation', 'current restart', 'previous rollback')[index]
                    prepared = json.loads(run(label + ' prepare',
                        [binary, 'prepare', '--home', home / '.codex', '--state', state]).stdout)
                    assert prepared['prepared'] is True
                    assert prepared['native_defaults_changed'] is False
                    assert prepared['app_inference_submitted'] is False
                    status = json.loads(run(label + ' local status', [binary, 'status', '--state', state]).stdout)
                    active = dict(binary=binary, pid=status['process']['pid'],
                                  start=str(status['process']['start']))
                    credential = run(label + ' credential', [binary, 'token', '--state', state], secret=True).stdout
                    if endpoint is not None:
                        assert status['base_url'] == endpoint, 'registered endpoint drift'
                        assert credential != original_credential, 'credential failed to rotate'
                    endpoint, original_credential = status['base_url'], credential
                    profile = Path(prepared['profile'])
                    assert str(binary) in profile.read_text()
                    assert credential.strip() not in profile.read_bytes()
                    retained_catalogs[Path(prepared['catalog'])] = sha(Path(prepared['catalog']))
                    # Upstream rejects --profile on `features list`; MCP list is
                    # an admitted configuration-loading command and does no work.
                    run(label + ' native --profile configuration',
                        [launcher, '--profile', 'chatgpt', 'mcp', 'list', '--json'])
                    assert all(sha(p) == digest for p, digest in sentinels.items())
                    assert sha(settings) == settings_hash
                    assert all(sha(p) == digest for p, digest in retained_catalogs.items())
                    assert not (state / 'desktop').exists(), 'model-free setup acquired an app'
                    assert not (case_root / 'state/codex/providers/chatgpt/desktop').exists()
                    stopped = json.loads(run(label + ' confirmed stop', [binary, 'stop', '--state', state]).stdout)
                    assert stopped.get('stopped') is True
                    active = None
                # Removing the optional Provider executable must not break native launch.
                unavailable = previous.with_name('unavailable')
                previous.rename(unavailable)
                run('native config load with absent optional Provider', [launcher, 'features', 'list'])
                run('direct native config with absent optional Provider',
                    [launcher, 'features', 'list'], selected=dict(env, CODEX_TERMUX_CORE_ONLY='1'))
                unavailable.rename(previous)
                if composition != 'base':
                    run('Manager account creation after Provider registration',
                        [launcher, 'termux', 'profile', 'create', 'g-product'])
                    managed_home = home / '.local/share/codex/manager/profiles/g-product/home'
                    assert managed_home.is_dir(), 'Manager account fixture not found'
                    run('select isolated Manager account',
                        [launcher, 'termux', 'profile', 'default', 'g-product'])
                    managed_env = {key: value for key, value in env.items() if key != 'CODEX_HOME'}
                    run('Manager account reads generated Provider configuration',
                        [launcher, '--profile', 'chatgpt', 'mcp', 'list', '--json'],
                        selected=managed_env)
                    assert (managed_home / 'chatgpt.config.toml').is_symlink()
                    assert (managed_home / 'chatgpt.config.toml').resolve() == profile
                case.update(accepted=True, native_config_and_auth_preserved=True,
                            stable_endpoint=True, credentials_rotated=True,
                            retained_catalogs=len(retained_catalogs), app_acquired=False,
                            fixture_manifest_sha256=sha(installed / 'release.manifest'))
            finally:
                if active is not None:
                    # Fence cleanup to the exact process/executable we just started.
                    proc = Path('/proc') / str(active['pid'])
                    if proc.exists():
                        identity = (proc / 'stat').read_text().rsplit(')', 1)[1].split()[19]
                        if identity == active['start'] and (proc / 'exe').resolve() == active['binary']:
                            result = subprocess.run([str(active['binary']), 'stop', '--state', str(state)],
                                                    env=env, capture_output=True, timeout=15)
                            case['cleanup_stop_exit'] = result.returncode
        assert {name: sha(getattr(args, name)) for name in
                ('core', 'frontend', 'provider', 'previous_provider')} == {
                    name: inputs[name] for name in ('core', 'frontend', 'provider', 'previous_provider')}
        assert {name: sha(source / name) for name in helpers} == inputs['helpers']
        report['accepted'] = True
    except Exception as error:
        report['failure'] = {'type': type(error).__name__, 'detail': str(error)}
    if report['accepted'] and not getattr(args, 'keep_payloads', False):
        retention = root / 'payload-cleanup.jsonl'
        try:
            report['payload_retention'] = dict(record=str(retention),
                                               **fixture_cleanup(retention, True, root))
        except Exception as error:
            # Qualification evidence remains valid; preserve incomplete cleanup
            # and make its failure explicit instead of discarding the root.
            report['payload_retention'] = dict(record=str(retention), error=type(error).__name__)
    report.update(ended_UTC=utc(), seconds=time.monotonic() - started)
    (root / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'accepted': report['accepted'], 'root': str(root),
                      'report': str(args.report), 'seconds': report['seconds'],
                      'failure': report.get('failure')}))
    return 0 if report['accepted'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--keep-payloads', action='store_true',
                        help='retain successful fixture executable copies for explicit debugging')
    for name in ('core', 'frontend', 'provider', 'previous-provider', 'generation', 'report'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    for name in ('core', 'frontend', 'provider', 'previous_provider', 'generation'):
        setattr(args, name, getattr(args, name).resolve(strict=True))
    # Never overwrite evidence or run fixtures outside the disposable Termux root.
    if args.report.exists():
        parser.error('report already exists; inspect its outcome before a new run')
    raise SystemExit(qualify(args))
