"""Bounded iterative-development/compaction proof, never an hours-long-work claim."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

CATALOG = 'def normalize_names(values):\n    return sorted(set(str(v).lower() for v in values if v))\n'
CLI = 'import json,sys\nfrom catalog import normalize_names\nprint(normalize_names(json.load(sys.stdin)))\n'
TESTS = '''import unittest,json,subprocess,sys
from catalog import normalize_names
class Contract(unittest.TestCase):
 def test_order_unicode(self):
  self.assertEqual(normalize_names([' Z ', 'Straße', 'STRASSE', ' z ', '', ' A  B ']),['z','strasse','a b'])
 def test_generator(self):
  self.assertEqual(normalize_names(x for x in ['B','a','B']),['b','a'])
 def test_type(self):
  with self.assertRaises(TypeError):normalize_names(['ok',3])
 def test_cli(self):
  p=subprocess.run([sys.executable,'cli.py'],input='[" Z ", "a"]',text=True,capture_output=True)
  self.assertEqual(p.returncode,0);self.assertEqual(json.loads(p.stdout),['z','a'])
 def test_cli_invalid(self):
  for text in ['{','[1]']:
   p=subprocess.run([sys.executable,'cli.py'],input=text,text=True,capture_output=True)
   self.assertEqual(p.returncode,2);self.assertIn('invalid input',p.stderr);self.assertNotIn('Traceback',p.stderr)
'''
README = '''# Name catalog
Improve this small Python library and stdin CLI to meet the contract.
normalize_names accepts an iterable of strings, strips and collapses whitespace,
uses Unicode casefold, omits empty names, deduplicates preserving first order,
and raises TypeError for any non-string entry. Do not coerce invalid entries.
cli.py reads one JSON list of strings from stdin and writes a JSON list plus newline.
Malformed JSON, non-list input or non-string entries exit2, stderr contains
"invalid input", with no traceback. Preserve test_catalog.py unchanged.
project-token.txt contains a synthetic project token: read it now and remember it
for continuation. It is NOT a credential. Do not copy it into source or tests.
Use python -m unittest -v test_catalog.py. Run the baseline before editing, inspect
real failures, fix implementations and run tests again. No packages are required.
'''


class Development:
    def __init__(self, output):
        self.output = output
        self.commands = []
        self.continuity = {}

    def configure(self, config, root, home, workspace, marker):
        self.marker = marker
        self.workspace = workspace
        for name, text in [('catalog.py', CATALOG), ('cli.py', CLI), ('test_catalog.py', TESTS), ('README.md', README), ('project-token.txt', marker+'\n')]:
            (workspace/name).write_text(text)
        self.protected = {name: hashlib.sha256((workspace/name).read_bytes()).hexdigest() for name in ['test_catalog.py', 'README.md']}
        return config

    def task(self, _marker):
        return ('Improve this disposable repository according to README.md. First say LOCAL_INITIAL_REPORT. '
            'Read the contract, implementations and synthetic project token; run the existing tests BEFORE editing, '
            'then implement, test and fix real failures until all requirements work. Preserve the supplied tests and README. '
            'Use native tools and real results; do not fabricate success. Use login=false for every exec_command: this test home must not load services. '
            'Useful intermediate public reports are welcome. Do not use the literal LOCAL_INTERMEDIATE_REPORT yet. '
            'No access to files outside the disposable workspace, no credentials/network/package installation. '
            'Remember the project token without copying it into source/tests. Final answer for this stage exactly DEVELOPMENT_STAGE_DONE.')

    def observe(self, row):
        item = row.get('params', {}).get('item', {})
        if row.get('method') == 'item/completed' and item.get('type') == 'commandExecution':
            self.commands.append({'exitCode': item.get('exitCode'), 'tests': 'unittest' in item.get('command', ''),
                'commandSha256': hashlib.sha256(item.get('command', '').encode()).hexdigest()})
            (self.output/'development-commands.json').write_text(json.dumps(self.commands, indent=2)+'\n')
            (self.output/'developed-source.json').write_text(json.dumps({n: (self.workspace/n).read_text() for n in ['catalog.py', 'cli.py'] if (self.workspace/n).exists()}, indent=2)+'\n')
            with (self.output/'command-output.jsonl').open('a') as log:
                log.write(json.dumps({'exitCode': item.get('exitCode'), 'output': item.get('aggregatedOutput', '')})+'\n')

    def respond(self, _row, _send):
        return False

    def continue_work(self, rpc, collect, thread, deadline, workspace, home, report, events):
        assert report['turnCompleted'], 'development_stage_failed'
        self.continuity['baselineTestFailed'] = any(c['tests'] and c['exitCode'] not in [0, None] for c in self.commands)
        self.continuity['regressionTestPassed'] = any(c['tests'] and c['exitCode'] == 0 for c in self.commands)
        assert self.continuity['baselineTestFailed'] and self.continuity['regressionTestPassed'], 'missing_real_test_failure_or_repair'
        assert all(hashlib.sha256((workspace/n).read_bytes()).hexdigest() == d for n, d in self.protected.items()), 'protected_fixture_changed'
        check = subprocess.run([sys.executable, '-B', '-c',
            'from catalog import normalize_names as f; assert f([" Q\\tR ","q r","Σ","σ","ς"])==["q r","σ"]; '
            'assert f(iter(["B","a","b"]))==["b","a"]'], cwd=workspace, capture_output=True, timeout=10)
        assert check.returncode == 0, 'independent_holdout_failed'
        self.continuity['independentHoldoutPassed'] = True
        self.source_hashes = {n: hashlib.sha256((workspace/n).read_bytes()).hexdigest() for n in ['catalog.py', 'cli.py']}
        self.source_stats = {n: ((workspace/n).stat().st_ino, (workspace/n).stat().st_mtime_ns, (workspace/n).stat().st_size) for n in self.source_hashes}
        # Only this verified generated token is removed: subsequent success must
        # come from actual tool history/checkpoint, not rereading the source.
        token = workspace/'project-token.txt'
        assert token.is_file() and not token.is_symlink() and token.read_text() == self.marker+'\n'
        token.unlink()
        before = len(self.commands)
        events.clear()
        rpc(101, 'thread/compact/start', {'threadId': thread}, deadline)
        collect()
        self.continuity['nativeCompactionCompleted'] = bool(report.get('warmCompactionStarted') and report.get('warmCompactionStarted') == report.get('warmCompactionCompleted'))
        summaries = [json.loads(line).get('payload', {}).get('message', '') for p in (home/'.codex/sessions').rglob('*.jsonl')
            for line in p.read_text().splitlines() if line.strip() and json.loads(line).get('type') == 'compacted']
        self.continuity['summaryRetainsToolOnlyToken'] = any(self.marker in text for text in summaries)
        assert self.continuity['nativeCompactionCompleted'] and self.continuity['summaryRetainsToolOnlyToken'], 'checkpoint_missing_tool_only_token'
        events.clear()
        rpc(102, 'turn/start', {'threadId': thread, 'input': [{'type': 'text', 'text':
            'Resume the completed development after compaction. Say LOCAL_INTERMEDIATE_REPORT. '
            'Do not redo edits or change catalog.py, cli.py, tests or README. Run the regression tests once. '
            'The token source file is gone: recall the project token only from completed tool history/checkpoint, '
            'write that exact value plus newline to proof.txt using a native command, then final answer exactly the token. '
            'Use login=false and do no other work.'}]}, deadline)
        collect()
        self.continuity['continuationRanTests'] = any(c['tests'] and c['exitCode'] == 0 for c in self.commands[before:])
        self.continuity['completedImplementationsNotRewritten'] = all(hashlib.sha256((workspace/n).read_bytes()).hexdigest() == d for n, d in self.source_hashes.items())

    def verify_cold(self, rpc, collect, thread, deadline, report, events):
        commands = len(self.commands)
        events.clear()
        rpc(103, 'turn/start', {'threadId': thread, 'input': [{'type': 'text', 'text':
            'After this Core restart, recall the synthetic project token from completed conversation/checkpoint. '
            'Do not call tools, read files or modify anything. Reply with exactly the project token.'}]}, deadline)
        collect()
        self.continuity['coldModelRemembersTokenWithoutTools'] = report['turnCompleted'] and report['nativeReplyMatched'] and len(self.commands) == commands

    def verify(self, root, home, workspace, items, report):
        results = dict(self.continuity)
        results['protectedTestsAndContractUnchanged'] = all(hashlib.sha256((workspace/n).read_bytes()).hexdigest() == d for n, d in self.protected.items())
        results['tokenFileAbsent'] = not (workspace/'project-token.txt').exists()
        results['projectTokenRecovered'] = (workspace/'proof.txt').read_text().strip() == self.marker
        results['completedImplementationIdentityPreserved'] = all(((workspace/n).stat().st_ino, (workspace/n).stat().st_mtime_ns, (workspace/n).stat().st_size) == v for n, v in self.source_stats.items())
        results['accepted'] = all(results.values())
        (self.output/'development-commands.json').write_text(json.dumps(self.commands, indent=2)+'\n')
        (self.output/'developed-source.json').write_text(json.dumps({n: (workspace/n).read_text() for n in ['catalog.py', 'cli.py']}, indent=2)+'\n')
        payloads = [r.get('payload', {}) for p in (home/'.codex/sessions').rglob('*.jsonl') for line in p.read_text().splitlines()
            if line.strip() for r in [json.loads(line)] if r.get('type') == 'response_item']
        (self.output/'native-tools.json').write_text(json.dumps([p for p in payloads if p.get('type') in ['function_call', 'function_call_output', 'custom_tool_call', 'custom_tool_call_output']], indent=2)+'\n')
        return results
