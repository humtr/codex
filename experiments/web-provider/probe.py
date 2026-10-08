"""Run a pinned public Core and real local tools in a wholly owned fixture."""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import sys
import tempfile
import threading

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
from qualify_tasks import fixture_activation_state, materialize_signed_generation
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest
from protocol import Transcript, events


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def qualify(generation, public_key, parent, core_sha, runtime_sha, capability_seed):
    headers, files = parse_manifest(generation / 'release.manifest')
    fields = parse_descriptor(generation / 'generation.meta')
    assert headers['generation_id'] == fields['generation_id']
    subprocess.run(['openssl', 'pkeyutl', '-verify', '-pubin', '-inkey', str(public_key),
                    '-rawin', '-in', str(generation / 'release.manifest'),
                    '-sigfile', str(generation / 'release.sig')], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for relative, expected, mode in files:
        source = generation / relative
        assert source.is_file() and not source.is_symlink() and digest(source) == expected, relative
        assert source.stat().st_mode & 0o777 == int(mode, 8), relative
    assert digest(generation / 'core') == core_sha
    assert digest(generation / 'runtime') == runtime_sha
    with tempfile.TemporaryDirectory(prefix='wp', dir=parent) as temporary:
        root = Path(temporary);home = root / 'h';prefix = root / 'p';account = home / '.codex';workspace = root / 'w'
        for path in [account, prefix / 'bin', prefix / 'etc/tls', home / '.local/share/codex/core/config', workspace]:
            path.mkdir(parents=True, exist_ok=True, mode=0o700)
        installed = home / '.local/lib/codex/core/generations' / fields['generation_id']
        materialize_signed_generation(generation, installed)
        launcher = prefix / 'bin/codex';shutil.copy2(generation / 'core', launcher)
        (prefix / 'bin/openssl').symlink_to(shutil.which('openssl'))
        (prefix / 'etc/resolv.conf').write_text('nameserver 127.0.0.1\n')
        (prefix / 'etc/tls/cert.pem').write_text('owned\n')
        (home / '.local/share/codex/core/activation-state').write_text(fixture_activation_state(fields['generation_id'], public_key))
        nonce = secrets.token_hex(16);(workspace / 'nonce.txt').write_text(nonce + '\n')
        transcript = Transcript(nonce)
        observations = []
        failures = []

        class Handler(http.server.BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                try:
                    if self.path != '/v1/responses':
                        raise ValueError('unexpected endpoint')
                    size = int(self.headers.get('Content-Length', '0'))
                    if not 0 < size <= 1024 * 1024:
                        raise ValueError('request size refused')
                    body = json.loads(self.rfile.read(size))
                    if body.get('model') != capability_seed or body.get('stream') is not True:
                        raise ValueError('fixture model or streaming mode changed')
                    transcript.receive(body)
                    observations.append({'path': self.path, 'model': body['model'],
                                         'input_types': [row['type'] for row in body['input']]})
                    if len(transcript.calls) < 2:
                        item = transcript.next_command(workspace, {key: env[key] for key in ['HOME', 'CODEX_HOME', 'PREFIX']})
                    else:
                        item = {'id': 'msg_owned', 'type': 'message', 'role': 'assistant', 'status': 'completed',
                                'content': [{'type': 'output_text', 'text': transcript.final_reply(), 'annotations': []}]}
                    payload = events(item, len(observations))
                    self.send_response(200);self.send_header('Content-Type', 'text/event-stream')
                    self.send_header('Content-Length', str(len(payload)));self.end_headers()
                    self.wfile.write(payload)
                except (ValueError, KeyError, AssertionError) as error:
                    failures.append({'error': type(error).__name__ + ': ' + str(error),
                                     'keys': sorted(body) if isinstance(body, dict) else [],
                                     'generate': body.get('generate'), 'model': body.get('model'),
                                     'input_types': [row.get('type') for row in body.get('input', [])],
                                     'tools_count': len(body.get('tools', [])),
                                     'tool_types': [{k: row.get(k) for k in ['type', 'name', 'execution']} for row in body.get('tools', [])],
                                     'declaration_types': [row.get('type') for row in body.get('input', []) if row.get('type') == 'additional_tools'],
                                     'owned_tool_results': [row.get('output') for row in body.get('input', []) if row.get('type') in ('function_call_output', 'custom_tool_call_output')]})
                    self.send_response(422);self.send_header('Content-Length', '0');self.end_headers()

        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        threading.Thread(target=server.serve_forever, daemon=True).start()
        config = f'''model = "{capability_seed}"
model_provider = "owned_probe"
approval_policy = "never"
sandbox_mode = "danger-full-access"
web_search = "disabled"
check_for_update_on_startup = false
[analytics]
enabled = false
[feedback]
enabled = false
[features]
memories = false
[model_providers.owned_probe]
name = "Owned protocol fixture"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
'''
        profile_config = account / 'web-probe.config.toml';profile_config.write_text(config)
        env = {'HOME': str(home), 'CODEX_HOME': str(account), 'PREFIX': str(prefix),
               'PATH': str(Path(shutil.which('sh')).parent), 'TMPDIR': str(parent),
               'SSL_CERT_FILE': str(prefix / 'etc/tls/cert.pem'), 'TERM': 'xterm-256color',
               'SHELL': '/data/data/com.termux/files/usr/bin/bash'}
        argv = [str(launcher), '-p', 'web-probe', 'exec', '--ephemeral', '--skip-git-repo-check',
                '--json', '-C', str(workspace), 'Read nonce.txt using local tools and verify it twice.']
        protected = {relative: digest(installed / relative) for relative in
                     [row[0] for row in files] + ['release.manifest', 'release.sig']}
        try:
            version = subprocess.run([str(launcher), '--version'], env=env, cwd=workspace,
                                     capture_output=True, timeout=15)
            assert version.returncode == 0 and version.stdout.decode() == 'codex-cli ' + fields['upstream_package_version'] + '\n'
            result = subprocess.run(argv, env=env, cwd=workspace, stdin=subprocess.DEVNULL,
                                    capture_output=True, text=True, timeout=45)
            stdout = [json.loads(line) for line in result.stdout.splitlines()]
            commands = [row['item'] for row in stdout if row.get('type') == 'item.completed'
                        and row.get('item', {}).get('type') == 'command_execution']
            assert not failures, failures
            assert result.returncode == 0, result.stderr
            assert len(observations) == 3 and len(commands) == 2
            assert all(row.get('exit_code') == 0 and row.get('aggregated_output') == nonce + '\n' for row in commands), commands
            assert transcript.final_reply() in result.stdout
            assert profile_config.read_text() == config
            assert (workspace / 'nonce.txt').read_text() == nonce + '\n'
            assert protected == {relative: digest(installed / relative) for relative in protected}
            assert not list(home.rglob('auth.json'))
            assert not (home / '.local/share/codex/core/servers').exists(), 'exec unexpectedly entered shared server'
            assert all(Path(env[key]).is_relative_to(root) for key in ['HOME', 'CODEX_HOME', 'PREFIX'])
            routes = transcript.routes
            return {'core_sha256': core_sha, 'runtime_sha256': runtime_sha,
                    'generation': fields['generation_id'], 'version': version.stdout.decode().strip(),
                    'argv': argv, 'execution_roots': {key: env[key] for key in ['HOME', 'CODEX_HOME', 'PREFIX']},
                    'requests': observations, 'tools': transcript.tool_definitions,
                    'custom_routes_observed': sum(row['type'] == 'custom' for row in routes.values()),
                    'namespaced_routes_observed': sum(namespace is not None for namespace, _ in routes),
                    'executed_routes': [{'name': row['name'], 'namespace': row.get('namespace')} for row in transcript.calls],
                    'real_service_model_verified': False, 'capability_seed': capability_seed + ' (loopback only)',
                    'proof': ['fully-owned-HOME-CODEX_HOME-PREFIX', 'exact-signed-artifact-version',
                              'actual-public-Core/config-layer/ephemeral-exec', 'two-local-command-executions',
                              'strict-call-ID-arguments-results', 'nonce-derived-final-reply',
                              'signed-generation/config/nonce-preserved', 'no-auth/no-shared-server']}
        finally:
            server.shutdown();server.server_close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ['generation', 'public-key', 'parent', 'report']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--capability-seed', choices=['gpt-5.5', 'gpt-5.6-sol'], required=True)
    parser.add_argument('--core-sha256', required=True);parser.add_argument('--runtime-sha256', required=True)
    args = parser.parse_args()
    proof = qualify(args.generation.resolve(), args.public_key.resolve(), args.parent.resolve(), args.core_sha256, args.runtime_sha256, args.capability_seed)
    args.report.write_text(json.dumps(proof, sort_keys=True, indent=2) + '\n')
    print(json.dumps({key: proof[key] for key in ['version', 'core_sha256', 'runtime_sha256', 'custom_routes_observed', 'namespaced_routes_observed', 'proof']}, sort_keys=True))
