"""Scoped installed or isolated candidate Native proof using the synthetic Operator.

The service's Bridge is synthetic; Core, Native ownership, execution metadata,
resource admission and clang are real. This is neither a live ChatGPT nor a TUI
acceptance. Run from an external foreground terminal, never inside a Native
opaque command whose account-wide claim would correctly block this test.
"""
import concurrent.futures
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import socket
import stat
import struct
import subprocess
import tempfile
import threading
import time
import urllib.request

from qualify_provider_operator import TOKEN, fixture_auth
from qualify_provider_native_trial import stop_owned_pid
from qualify_tasks import Ws, fixture_activation_state, local_generation_inventory

FUNCTIONS = 32768
MAX_POLLS = 30


def stamp():
    return {'utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'monotonic_ns': time.monotonic_ns()}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def termux_path(path, home, prefix):
    path = Path(path).absolute()
    assert any(path.is_relative_to(base) for base in (home, prefix)), 'fixture inputs and output must remain inside Termux'
    resolved = path.resolve(strict=True)
    assert any(resolved.is_relative_to(base) for base in (home, prefix)), 'fixture path aliases outside Termux'
    return resolved


def plain_source(label):
    assert label in ('a', 'b')
    # Finite, ordinary C gives both real processes time to yield. No wrapper,
    # sleep, stdin pipe, environment override or compiler-plugin exception.
    return ''.join(f'int {label}{index}(int x) {{ return x + {index}; }}\n'
                   for index in range(FUNCTIONS)) + 'int main(void) { return 0; }\n'


def payload(root, domain, identity):
    key = hashlib.sha256(json.dumps([domain, identity], separators=(',', ':')).encode()).hexdigest()
    return json.loads((root / 'provider-state' / (key + '.json')).read_text())['payload']


def execution_proof(record, turn, *, home, prefix, workspace, runtime, observed):
    owner = record['execution']
    proof = turn.get('execution')
    assert isinstance(proof, dict), 'actual Native execution metadata is absent; no fixture substitute is allowed'
    assert owner['process'] == observed, 'Native process identity changed'
    assert observed['image'] == str(runtime), 'selected Native owner is not the explicit installed runtime'
    assert owner['home'] == str(workspace.parent / 'h/.codex')
    assert Path(owner['directory']).parent == home / '.local/share/codex/core/catalogs'
    assert type(proof['schema']) is int and proof['schema'] == 4, 'actual dispatch-bound Native hook proof is required'
    tools = proof.get('hook_free_tools')
    assert isinstance(tools, list) and len(tools) <= 6 and all(isinstance(tool, str) and tool in (
        'exec_command', 'write_stdin', 'apply_patch', 'view_image', 'clock.curr_time', 'clock.sleep'
    ) for tool in tools) and len(set(tools)) == len(tools) and 'exec_command' in tools, 'actual Native tool hook proof is absent'
    assert proof['pid'] == observed['pid'] and proof['start'] == observed['start']
    assert proof['thread'] == record['thread'] and proof['turn'] == record['native_turn']
    assert proof['cwd'] == str(workspace) and proof['prefix'] == str(prefix)
    assert proof['home'] == str(home) and proof['shell'] == str(prefix / 'bin/bash')
    assert proof['no_default_config'] is False, 'this case must exercise the installed default-config proof'
    assert 'CLANG_NO_DEFAULT_CONFIG' not in proof['variables'], 'do not force a different compiler configuration'
    return proof


def process(owner):
    pid = owner['process']['pid']
    raw = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
    assert raw[0] not in ('Z', 'X'), 'Native owner departed'
    return {'pid': pid, 'start': raw[19], 'image': os.readlink(f'/proc/{pid}/exe')}


def original_result(reply, invocation):
    assert invocation['offered'] is True and invocation['result'] is not None
    result = reply['result']
    assert result == invocation['result'], 'original Native result changed at the Operator boundary'
    assert result['call_id'] == invocation['id']
    assert result['type'] == 'function_call_output' and isinstance(result['output'], str)
    return result


def status_header(result):
    return result['output'].partition('\nOutput:\n')[0]


def completion_proof(result, outcome, session=None):
    """Read only the original harness's first frame, never command stdout."""
    def unique(fields):
        value = {}
        for key, item in fields:
            assert key not in value, 'duplicate Native completion field'
            value[key] = item
        return value

    first = result['output'].split('\n', 1)[0]
    prefix = 'Native completion: '
    assert first.startswith(prefix), 'actual Native completion frame is missing'
    proof = json.loads(first[len(prefix):], object_pairs_hook=unique)
    assert isinstance(proof, dict) and set(proof) == {
        'schema', 'call_id', 'session_id', 'outcome', 'exit_code', 'hooks'
    }, 'actual Native completion is unknown or malformed'
    assert proof['schema'] == 'codex-native-completion-v1'
    assert proof['call_id'] == result['call_id'], 'completion belongs to another invocation'
    assert proof['hooks'] == 'absent'
    assert proof['session_id'] is None or (
        type(proof['session_id']) is int and 0 <= proof['session_id'] <= 2**31 - 1)
    if session is not None:
        assert proof['session_id'] == session, 'completion belongs to another Native session'
    assert proof['outcome'] == outcome, 'parent status is not whole-process-tree completion'
    if outcome == 'session_running':
        assert proof['session_id'] is not None and proof['exit_code'] is None
    else:
        assert outcome == 'process_tree_completed'
        assert type(proof['exit_code']) is int and proof['exit_code'] == 0, 'compiler tree did not complete successfully'
    return proof


def running_session(result, *, allow_parent_exit=False):
    header = status_header(result)
    matches = re.findall(r'^Process (?:running|scope remains active) with session ID ([0-9]+)$', header, re.MULTILINE)
    assert len(matches) == 1, 'actual compiler did not yield a retained session; parallel execution is unproven'
    if not allow_parent_exit:
        assert not re.search(r'(?:Process exited with code|Exit code:)\s*-?\d+', header)
    return int(matches[0])


def completed(result):
    header = status_header(result)
    codes = re.findall(r'^(?:Process exited with code |Exit code: )(-?\d+)\s*$', header, re.MULTILINE)
    if not codes:
        return False
    assert codes == ['0'], 'actual Native command failed'
    return not re.search(r'^Process (?:running|scope remains active) with session ID ', header, re.MULTILINE)


def command_output(result):
    text = result['output']
    assert isinstance(text, str), 'original Native output envelope is missing'
    _, delimiter, output = text.partition('\nOutput:\n')
    assert delimiter, 'original Native execution output framing is missing'
    return output


def compiler_claim(invocation, workspace, label, digest):
    effect = invocation.get('effect')
    paths = effect.get('Paths') if isinstance(effect, dict) else None
    assert isinstance(paths, list), 'compiler was not admitted with bounded path claims'
    inputs = [item for item in paths if item['path'] == str(workspace / f'{label}.c')]
    outputs = [item for item in paths if item['path'] == str(workspace / f'{label}.out')]
    assert len(inputs) == len(outputs) == 1
    assert inputs[0]['operation'] == 'Input' and inputs[0]['base'] == digest
    assert outputs[0]['operation'] == 'Add' and outputs[0]['base'] is None
    assert any(item.get('external_input') for item in paths), 'installed execution dependencies were not pinned'


def simultaneous_claims(status, expected):
    active = status['native_effects']['active']
    rows = [row for row in active if row['owner_work'] in expected]
    assert len(rows) == 2 and len({row['owner_work'] for row in rows}) == 2
    for row in rows:
        assert row['call'] == expected[row['owner_work']]['id']
        assert row['process_session'] == expected[row['owner_work']]['session']
        assert row['resource_kind'] == 'bounded' and row['pending_result'] is True
    return rows


def command_item(thread, command, shell, workspace, call_id, output):
    items = [item for turn in thread['turns'] for item in turn['items']
             if item.get('type') == 'commandExecution']
    assert len(items) == 1, 'Native command must execute exactly once in its own thread'
    item = items[0]
    assert item['id'] == call_id and item['status'] == 'completed'
    assert type(item['exitCode']) is int and item['exitCode'] == 0
    assert item['cwd'] == str(workspace)
    assert shlex.split(item['command']) == [str(shell), '-c', command]
    # Native v2 item_builders and CoreTurnItem conversion publish null for a
    # proven empty aggregate. Bind this to the original handler receipt, never
    # default a missing/invalid history result to empty output.
    assert isinstance(output, str)
    assert 'aggregatedOutput' in item
    assert item['aggregatedOutput'] == (output if output else None)
    assert all(type(item.get(key)) is int for key in ('startedAtMs', 'completedAtMs'))
    assert item['startedAtMs'] <= item['completedAtMs']
    return item


def published_command(thread, command, shell, workspace, call_id, output):
    items = [item for turn in thread['turns'] for item in turn['items']
             if item.get('type') == 'commandExecution']
    assert len(items) <= 1, 'Native command executed more than once'
    if not items:
        return None
    assert items[0]['id'] == call_id, 'Native published a different command'
    if items[0]['status'] == 'inProgress':
        return None
    if items[0]['status'] == 'completed' and any(items[0].get(key) is None for key in ('startedAtMs', 'completedAtMs')):
        return None
    return command_item(thread, command, shell, workspace, call_id, output)


def overlapping_commands(items):
    overlap = min(item['completedAtMs'] for item in items) - max(item['startedAtMs'] for item in items)
    assert len(items) == 2 and overlap > 0, 'actual Native command intervals did not overlap'
    return overlap


class Client:
    def __init__(self, port, root):
        self.port, self.root, self.sequence = port, root, 0
        self.lock = threading.Lock()

    def call(self, name, arguments):
        with self.lock:
            self.sequence += 1
            identity = self.sequence
        request = urllib.request.Request(f'http://127.0.0.1:{self.port}/mcp',
            data=json.dumps({'jsonrpc': '2.0', 'id': identity, 'method': 'tools/call',
                             'params': {'name': name, 'arguments': arguments}}).encode(),
            headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + TOKEN})
        # This fixture endpoint is loopback even when the invoking terminal has
        # a proxy configured. Compiler execution still inherits the actual env.
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        with opener.open(request, timeout=45) as response:
            value = json.load(response)
        save(self.root / f'rpc-{identity}.json', value)
        assert value.get('id') == identity and 'error' not in value, f'RPC {identity} failed; original receipt retained'
        result = value['result']
        assert result['isError'] is False, f'{name} failed; inspect private RPC {identity}'
        return json.loads(result['content'][0]['text'])


def confirmed_native_stop(value):
    """A successful RPC is not confirmation that its original effects stopped."""
    return (isinstance(value, dict) and value.get('native_stop_confirmed') is True
            and value.get('remaining_jobs') == []
            and (value.get('host_stop_requested') is False
                 or (value.get('host_stop_requested') is True
                     and value.get('host_stop_confirmed') is True)))


def stop_fixture_works(client, works, root, *, creation_pending=False):
    """Stop only these original fixture works once; retain uncertain owners."""
    if creation_pending:
        # Earlier known works do not account for an unanswered later create.
        return ['fixture_work_admission_unconfirmed']
    if client is None:
        return ['fixture_client_unavailable'] if works else []
    if not works:
        # An unanswered allocation may have created a work. A missing local
        # receipt is not authority to terminate its result-producing service.
        return ['fixture_work_admission_unconfirmed']
    errors = []
    for label, work in works.items():
        try:
            stopped = client.call('codex_stop', {'work_id': work['work_id']})
            save(root / f'cleanup-stop-{label}.json', stopped)
            assert confirmed_native_stop(stopped), 'fixture Native/host Stop remains unconfirmed'
        except Exception as error:
            errors.append(type(error).__name__)
            save(root / f'cleanup-stop-{label}-error.json',
                 {'error_type': type(error).__name__, 'original_work_preserved': True})
    if not errors:
        try:
            work = next(iter(works.values()))
            status = client.call('codex_status', {'work_id': work['work_id']})
            save(root / 'cleanup-status.json', status)
            assert status.get('work_id') == work['work_id'] and status.get('thread_id') == work['thread_id']
            assert status.get('phase') == 'stopped'
            effects = status.get('native_effects', {})
            assert effects.get('active') == [] and type(effects.get('unconfirmed_effects')) is int and effects['unconfirmed_effects'] == 0
        except Exception as error:
            errors.append(type(error).__name__)
            save(root / 'cleanup-status-error.json',
                 {'error_type': type(error).__name__, 'original_work_preserved': True})
    return errors


def cleanup_compiler_fixture(client, works, service, root, accepted, owners, native_home, runtime, *, creation_pending=False):
    errors = [] if accepted and not creation_pending else stop_fixture_works(client, works, root, creation_pending=creation_pending)
    if errors:
        # The same Provider and Native owners remain available for their
        # existing public Stop/reconciliation path. No retry or replacement.
        save(root / 'cleanup-unconfirmed.json', {'errors': errors, 'original_owners_preserved': True})
        return errors
    if service is not None and service.poll() is None:
        (root / 'stop-provider').touch()
        try:
            service.wait(timeout=30)
        except subprocess.TimeoutExpired:
            service.terminate()
            service.wait(timeout=10)
    if accepted:
        # Only our unique execution home, after all original calls/finals
        # settled. Never stop an installed user's existing Native owner.
        for owner in owners.values():
            assert owner['home'] == str(native_home) and process(owner) == owner['process']
            save(root / f'owned-owner-stop-{owner["process"]["pid"]}.json',
                 stop_owned_pid(owner['process']['pid'], runtime, owner['process']['start']))
    return errors


def selected_installation(core, generation, home, prefix, candidate_root=None):
    """Keep operating acceptance strict; candidates use an explicit private fixture."""
    scope = 'installed-native-compiler'
    if candidate_root is None:
        assert core == (prefix / 'bin/codex').resolve(strict=True), 'use the actual installed Core entrypoint'
    else:
        candidate = Path(candidate_root).absolute()
        assert candidate == termux_path(candidate, home, prefix), 'candidate root contains an alias'
        assert home == candidate / 'h', 'candidate HOME must be the owned fixture home'
        assert core == candidate / 'p/bin/codex', 'candidate Core must be the owned fixture entrypoint'
        metadata = core.lstat()
        assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.geteuid() and metadata.st_mode & 0o111 and not metadata.st_mode & 0o022, 'candidate Core executable is unsafe'
        for directory in (candidate, home):
            metadata = directory.lstat()
            assert stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == os.geteuid() and not metadata.st_mode & 0o077, 'candidate directory is not private and owned'
        marker = (candidate / 'owned-fixture').lstat()
        assert stat.S_ISREG(marker.st_mode) and marker.st_uid == os.geteuid() and not marker.st_mode & 0o077, 'owned candidate marker is unavailable'
        public = candidate / 'fixture-public.pem'
        metadata = public.lstat()
        assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.geteuid() and not metadata.st_mode & 0o022, 'fixture public key is unsafe'
        assert generation.parent == home / '.local/lib/codex/core/generations'
        fields, _ = local_generation_inventory(generation)
        assert sha(core) == fields['core_artifact_digest'] == sha(generation / 'core'), 'candidate Core digest changed'
        assert sha(generation / 'runtime') == fields['runtime_digest'], 'candidate Runtime digest changed'
        activation = home / '.local/share/codex/core/activation-state'
        metadata = activation.lstat()
        assert stat.S_ISREG(metadata.st_mode) and metadata.st_uid == os.geteuid() and not metadata.st_mode & 0o077, 'candidate activation is not private and owned'
        assert activation.read_text() == fixture_activation_state(generation.name, public), 'candidate activation must use only its ephemeral fixture key'
        subprocess.run(['openssl', 'pkeyutl', '-verify', '-pubin', '-inkey', str(public), '-rawin',
            '-in', str(generation / 'release.manifest'), '-sigfile', str(generation / 'release.sig')],
            check=True, capture_output=True, timeout=15)
        scope = 'candidate-native-compiler'
    assert generation.parent == home / '.local/lib/codex/core/generations'
    activation = dict(line.split('=', 1) for line in (home / '.local/share/codex/core/activation-state').read_text().splitlines() if '=' in line)
    assert activation['current'] == generation.name, 'explicit generation is not currently selected'
    return scope


def probe(args):
    os.umask(0o077)
    home, prefix = Path(os.environ['HOME']).resolve(strict=True), Path(os.environ['PREFIX']).resolve(strict=True)
    core, generation = termux_path(args.core, home, prefix), termux_path(args.generation, home, prefix)
    runtime, executable = generation / 'runtime', termux_path(args.service_test, home, prefix)
    scope = selected_installation(core, generation, home, prefix, getattr(args, 'candidate_root', None))
    assert runtime.is_file() and not runtime.is_symlink()
    assert os.environ.get('SHELL') == str(prefix / 'bin/bash'), 'this case requires the installed selected Bash'
    assert not os.environ.get('CLANG_NO_DEFAULT_CONFIG'), 'retain the actual default compiler configuration'
    assert not any(os.environ.get(key) for key in ('CODEX_PROVIDER_OPERATOR_PORT', 'CODEX_PROVIDER_OPERATOR_BASE_URL',
        'CODEX_PROVIDER_LIVE_DESCRIPTOR', 'CODEX_PROVIDER_LIVE_CHAT', 'CODEX_PROVIDER_OWNED_STATE'))
    args.report = args.report.absolute()
    args.report = termux_path(args.report.parent, home, prefix) / args.report.name
    assert not os.path.lexists(args.report), 'preserve the existing qualification report'
    temporary = termux_path(os.environ['TMPDIR'], home, prefix)
    # Exclusive launch before any service or Native allocation. A failed run is
    # inspected/reconciled, never silently rerun under the same report name.
    with args.report.with_suffix('.launch.json').open('x') as stream:
        json.dump({'scope': scope, 'model_requests': 0, **stamp()}, stream)
    root = Path(tempfile.mkdtemp(prefix='nc-', dir=temporary)).resolve(strict=True)
    workspace, native_home = root / 'w', root / 'h/.codex'
    for path in (workspace, native_home):
        path.mkdir(parents=True, mode=0o700)
    (root / 'owned-fixture').touch()
    # Same isolated Native policy as the existing synthetic Operator fixture;
    # the installed Core still enforces its real system requirements.
    (native_home / 'config.toml').write_text('approval_policy="never"\napprovals_reviewer="user"\n'
        'sandbox_mode="danger-full-access"\ncheck_for_update_on_startup=false\n'
        '[features]\nmemories=false\n[projects.' + json.dumps(str(workspace)) + ']\ntrust_level="trusted"\n')
    (native_home / 'auth.json').write_text(json.dumps({'OPENAI_API_KEY': 'owned-inert-fixture', 'auth_mode': 'apikey'}))
    configuration = root / 'operator.toml'
    configuration.write_text('version=1\n[transport]\nreturn_path="mcp"\n[execution]\nmode="operator"\n'
        + ''.join(key + '=' + json.dumps(str(value)) + '\n' for key, value in
                  [('core', core), ('home', native_home), ('workspace', workspace)]))
    fixture_auth(root / 'fixture-auth')
    # Keep the actual installed shell/PATH/compiler environment. Do not erase
    # unsafe compiler variables or invent metadata to make a claim pass.
    env = dict(os.environ, CODEX_HOME=str(native_home), CODEX_PROVIDER_OPERATOR_FIXTURE=str(root),
               CODEX_PROVIDER_OPERATOR_CONFIG=str(configuration), OPENAI_BASE_URL='http://127.0.0.1:9/v1',
               OPENAI_API_KEY='owned-inert-fixture')
    sources = {}
    for label in ('a', 'b'):
        path = workspace / f'{label}.c'
        path.write_text(plain_source(label))
        sources[label] = sha(path)
    save(root / 'inputs.json', {'core': {'path': str(core), 'sha256': sha(core)},
        'runtime': {'path': str(runtime), 'sha256': sha(runtime)},
        'service_test': {'path': str(executable), 'sha256': sha(executable)},
        'scope': scope, 'generation_manifest_sha256': sha(generation / 'release.manifest'),
        'configuration_sha256': sha(configuration), 'native_configuration_sha256': sha(native_home / 'config.toml'),
        'driver_sha256': sha(Path(__file__)),
        'sources': sources, 'functions_per_source': FUNCTIONS, 'home': str(home), 'prefix': str(prefix),
        'execution_home': str(native_home), 'workspace': str(workspace), 'model_requests': 0, **stamp()})
    save(args.report, {'accepted': False, 'phase': 'prepared', 'scope': scope,
                      'fixture': str(root), 'model_requests': 0, 'live_tui_accepted': False, **stamp()})
    service = client = None
    works, owners = {}, {}
    creation_pending = False
    accepted = False
    proof_result = None
    log = (root / 'provider-test.log').open('wb')
    try:
        service = subprocess.Popen([str(executable), '--ignored', '--exact', 'synthetic_operator_service', '--nocapture'],
                                   env=env, cwd=workspace, stdout=log, stderr=subprocess.STDOUT)

        def wait(predicate, description, seconds=40):
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                assert service.poll() is None, 'synthetic service exited; private log retained'
                value = predicate()
                if value:
                    return value
                time.sleep(.02)
            raise AssertionError(description + '; original work retained at ' + str(root))

        wait(lambda: (root / 'provider-ready.json').exists(), 'service readiness')
        client = Client(json.loads((root / 'provider-ready.json').read_text())['port'], root)

        def record(work):
            return payload(root, 'operator-work-v1', work['work_id'])

        def turn(work):
            return payload(root, 'mcp-turn-v1', record(work)['token'])

        def binding(work):
            return {key: work[key] for key in ('work_id', 'controller')}

        def native(work, method, params):
            owner = record(work)['execution']
            assert process(owner) == owner['process']
            assert owner['home'] == str(native_home) and owner['process']['image'] == str(runtime)
            connection = Ws(str(Path(owner['directory']) / 's'), uri_path='/rpc')
            try:
                pid, uid, _ = struct.unpack('3i', connection.socket.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                assert pid == owner['process']['pid'] and uid == os.geteuid()
                return connection.call(method, params)
            finally:
                connection.close()

        def command_published(work, command, call_id, output):
            # write_stdin's original terminal result precedes the exit watcher's
            # ItemCompleted publication. Read that same item until published;
            # never submit another exec or poll after terminal output arrived.
            def read():
                value = native(work, 'thread/read', {'threadId': work['thread_id'], 'includeTurns': True})['thread']
                item = published_command(value, command, prefix / 'bin/bash', workspace, call_id, output)
                return (value, item) if item is not None else None
            return wait(read, 'original Native command completion publication')

        for label in ('a', 'b', 'opaque'):
            creation_pending = True
            work = client.call('codex_sessions', {'action': 'create', 'inference': 'attached', 'call_id': 'compiler-create-' + label})
            works[label] = work
            creation_pending = False
            assert work['native_permissions']['approval_policy'] == 'never'
            assert work['native_permissions']['sandbox']['type'] == 'dangerFullAccess'
            client.call('codex_sessions', {**binding(work), 'action': 'start', 'call_id': 'compiler-start-' + label,
                'task': 'Owned model-free Native compiler qualification ' + label + ': run only the finite supplied compiler/resource checks and preserve all original results.'})

            def context():
                result = client.call('codex_attach', binding(work))
                return None if result.get('delivery') in ('pending_native_history', 'pending_native_context') else result

            receipt = wait(context, 'attached Native context')
            assert client.call('codex_attach', {**binding(work), 'accept': receipt['receipt']})['accepted'] is True
            catalog = client.call('codex_tools', binding(work))['tools']
            work['exec'] = next(key for key in catalog if key.rsplit('.', 1)[-1] == 'exec_command')
            work['poll'] = next(key for key in catalog if key.rsplit('.', 1)[-1] == 'write_stdin')
            saved = record(work)
            proof = execution_proof(saved, turn(work), home=home, prefix=prefix, workspace=workspace,
                                    runtime=runtime, observed=process(saved['execution']))
            assert 'write_stdin' in proof['hook_free_tools'], 'this fixture must collect original results through a supported continuation'
            owners[saved['execution']['process']['pid']] = saved['execution']
            save(root / f'execution-{label}.json', {'owner': saved['execution'], 'metadata': proof})
        assert len({work['thread_id'] for work in works.values()}) == 3

        def execute(work, identity, name, value):
            return client.call('codex_execute', {**binding(work), 'call_id': identity, 'name': name,
                'input': json.dumps(value), 'report': 'Owned Native compiler qualification ' + identity})

        arguments = {label: {'cmd': f'clang -O0 {label}.c -o {label}.out', 'login': False,
                            'workdir': str(workspace), 'yield_time_ms': 250, 'max_output_tokens': 1000} for label in ('a', 'b')}
        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
            futures = {label: pool.submit(execute, works[label], 'build-' + label, works[label]['exec'], arguments[label])
                       for label in ('a', 'b')}
            original = {label: future.result(timeout=45) for label, future in futures.items()}
        claims = {}
        invocations = {}
        for label in ('a', 'b'):
            invocation = turn(works[label])['calls']['build-' + label]
            compiler_claim(invocation, workspace, label, sources[label])
            result = original_result(original[label], invocation)
            assert command_output(result) == '', 'plain compiler unexpectedly produced output'
            invocations[label] = invocation
            claims[works[label]['work_id']] = {'id': invocation['id'], 'session': running_session(result)}
        status = client.call('codex_status', {'work_id': works['a']['work_id']})
        simultaneous_claims(status, claims)
        save(root / 'simultaneous-bounded-claims.json', {**status, **stamp()})
        opaque = works['opaque']
        opaque_args = {'cmd': 'printf compiler-settled >> opaque-effect', 'login': False,
                       'workdir': str(workspace), 'max_output_tokens': 1000}
        refused = execute(opaque, 'opaque-before-settlement', opaque['exec'], opaque_args)
        assert refused['result']['error'] == 'native_resource_busy' and refused['result']['executed'] is False
        assert refused['result']['retryable'] is True and not (workspace / 'opaque-effect').exists()
        assert turn(opaque)['calls']['opaque-before-settlement']['offered'] is False
        assert not any(item.get('type') == 'commandExecution' for row in native(opaque, 'thread/read',
            {'threadId': opaque['thread_id'], 'includeTurns': True})['thread']['turns'] for item in row['items'])

        def settle(label):
            work = works[label]
            session = claims[work['work_id']]['session']
            for index in range(MAX_POLLS):
                identity = f'build-{label}-poll-{index}'
                reply = execute(work, identity, work['poll'], {'session_id': session, 'chars': '',
                    'yield_time_ms': 1000, 'max_output_tokens': 1000})
                result = original_result(reply, turn(work)['calls'][identity])
                assert command_output(result) == '', 'plain compiler unexpectedly produced output'
                if completed(result):
                    completion_proof(result, 'process_tree_completed', session)
                    return result
                assert running_session(result, allow_parent_exit=True) == session, 'continuation changed the original Native process'
                completion_proof(result, 'session_running', session)
            raise AssertionError('finite compiler observation budget exhausted; no replacement execution')

        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
            settled = dict(zip(('a', 'b'), pool.map(settle, ('a', 'b'))))
        wait(lambda: client.call('codex_status', {'work_id': works['a']['work_id']})['native_effects']['unconfirmed_effects'] == 0,
             'original terminal results must release both claims')
        history, commands, outputs = {}, [], {}
        for label in ('a', 'b'):
            work = works[label]
            history[label], item = command_published(work, arguments[label]['cmd'], invocations[label]['id'], command_output(settled[label]))
            commands.append(item)
            assert (workspace / f'{label}.out').read_bytes().startswith(b'\x7fELF')
            assert stat.S_IMODE((workspace / f'{label}.out').stat().st_mode) & 0o111
            outputs[label] = {'path': str(workspace / f'{label}.out'), 'sha256': sha(workspace / f'{label}.out'),
                              'bytes': (workspace / f'{label}.out').stat().st_size}
            assert sha(workspace / f'{label}.c') == sources[label]
            assert execute(work, 'build-' + label, work['exec'], arguments[label])['result'] == original[label]['result']
            assert turn(work)['calls']['build-' + label]['result'] == invocations[label]['result']
        overlap = overlapping_commands(commands)
        assert execute(opaque, 'opaque-before-settlement', opaque['exec'], opaque_args)['result'] == refused['result']
        assert not (workspace / 'opaque-effect').exists(), 'rejected call silently ran after settlement'
        after = execute(opaque, 'opaque-after-settlement', opaque['exec'], opaque_args)
        opaque_result = original_result(after, turn(opaque)['calls']['opaque-after-settlement'])
        if not completed(opaque_result):
            opaque_session = running_session(opaque_result, allow_parent_exit=True)
            for index in range(MAX_POLLS):
                identity = f'opaque-after-settlement-poll-{index}'
                reply = execute(opaque, identity, opaque['poll'], {'session_id': opaque_session, 'chars': '',
                    'yield_time_ms': 1000, 'max_output_tokens': 1000})
                opaque_result = original_result(reply, turn(opaque)['calls'][identity])
                if completed(opaque_result):
                    completion_proof(opaque_result, 'process_tree_completed', opaque_session)
                    break
                assert running_session(opaque_result, allow_parent_exit=True) == opaque_session
                completion_proof(opaque_result, 'session_running', opaque_session)
            else:
                raise AssertionError('opaque scope observation budget exhausted; original execution retained')
        completion_proof(opaque_result, 'process_tree_completed')
        assert command_output(opaque_result) == '', 'redirected command unexpectedly produced output'
        assert (workspace / 'opaque-effect').read_text() == 'compiler-settled'
        _, opaque_item = command_published(opaque, opaque_args['cmd'], turn(opaque)['calls']['opaque-after-settlement']['id'], command_output(opaque_result))
        for label, work in works.items():
            client.call('codex_finish', {**binding(work), 'call_id': 'compiler-finish-' + label,
                'text': 'Owned Native compiler qualification completed with original Native results preserved.'})
            wait(lambda: client.call('codex_status', {'work_id': work['work_id']})['completed'], 'Native final settlement')
            final = native(work, 'thread/read', {'threadId': work['thread_id'], 'includeTurns': True})['thread']
            assert all(row['status'] == 'completed' for row in final['turns'])
            if label in history:
                assert command_item(final, arguments[label]['cmd'], prefix / 'bin/bash', workspace, invocations[label]['id'], command_output(settled[label])) == commands[('a', 'b').index(label)]
            else:
                assert command_item(final, opaque_args['cmd'], prefix / 'bin/bash', workspace,
                                    turn(work)['calls']['opaque-after-settlement']['id'], command_output(opaque_result)) == opaque_item
            save(root / f'native-history-{label}.json', final)
        final_status = client.call('codex_status', {'work_id': works['a']['work_id']})
        assert final_status['native_effects']['unconfirmed_effects'] == 0
        save(root / 'settled-status.json', final_status)
        expected_threads = {work['thread_id'] for work in works.values()}
        for pid in owners:
            work = next(work for work in works.values() if record(work)['execution']['process']['pid'] == pid)
            loaded = native(work, 'thread/loaded/list', {'limit': 100})
            assert not loaded.get('nextCursor') and set(loaded['data']) <= expected_threads
            save(root / f'owned-loaded-threads-{pid}.json', loaded)
        bridge_log = root / 'bridge-commands.json'
        assert not bridge_log.exists() or not any(row['op'] == 'submit' for row in json.loads(bridge_log.read_text()))
        save(root / 'original-results.json', {'initial': original, 'terminal': settled, 'opaque_rejected': refused,
            'opaque_completed': after, 'opaque_terminal': opaque_result})
        completions = {label: completion_proof(result, 'process_tree_completed') for label, result in settled.items()}
        completions['opaque'] = completion_proof(opaque_result, 'process_tree_completed')
        save(root / 'native-process-tree-completions.json', completions)
        proof_result = {'accepted': True, 'scope': scope, 'fixture': str(root),
            'model_requests': 0, 'live_tui_accepted': False, 'actual_compilers': 2,
            'outputs': outputs,
            'native_command_overlap_ms': overlap, 'simultaneous_bounded_claims': 2,
            'opaque_pre_effect_rejected': True, 'rejected_call_not_replayed': True,
            'original_results_preserved': True, 'whole_process_trees_completed': len(completions),
            'unconfirmed_effects': 0, **stamp()}
        accepted = True
    except BaseException as error:
        save(args.report, {'accepted': False, 'scope': scope, 'fixture': str(root),
                          'model_requests': 0, 'live_tui_accepted': False,
                          'error_type': type(error).__name__, 'error': str(error), **stamp()})
        raise
    finally:
        cleanup_compiler_fixture(client, works, service, root, accepted, owners, native_home, runtime, creation_pending=creation_pending)
        log.close()
        print('Owned Native compiler proof retained at ' + str(root), flush=True)
    save(args.report, proof_result)
