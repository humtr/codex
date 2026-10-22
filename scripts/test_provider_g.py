import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from scripts import qualify_provider_g as g
from scripts.qualify_permission_lifecycle import server_paths


class GDriver(unittest.TestCase):
    def test_installed_runtime_discovery_requires_the_exact_selected_image(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as name:
            home=Path(name)
            directory=home/'.local/share/codex/core/catalogs/owned'
            directory.mkdir(parents=True)
            (directory/'s').touch()
            (directory/'pid').write_text(str(os.getpid()))
            actual=Path('/proc/self/exe').resolve()
            self.assertEqual(server_paths(home,actual),[directory/'s'])
            self.assertEqual(server_paths(home,home/'different-runtime'),[])
            self.assertEqual(server_paths(home),[])

    def test_connector_readiness_requires_actual_remote_tools_and_read_call(self):
        info={'expected_tools':['codex_sessions','native_tool']}
        proof={'refresh':True,'cache_refresh':True,'live_mcp_probe':{'accepted':True,'model_requests':0,'native_effects':0,'samples':[{'read_call':True,'names':['native_tool','codex_sessions']}]}}
        g.require_connector_ready(info,proof)
        for change in [dict(refresh=False),dict(cache_refresh=False),dict(live_mcp_probe={'accepted':False})]:
            with self.assertRaises(AssertionError):g.require_connector_ready(info,dict(proof,**change))
        proof['live_mcp_probe']['samples'][0]['read_call']=False
        with self.assertRaises(AssertionError):g.require_connector_ready(info,proof)

    def test_cold_budget_extends_only_elapsed_cap_without_resetting_original_start(self):
        budget=dict(max_parallel=1,max_app_starts=4,max_native_calls=40,max_elapsed_seconds=900)
        record=dict(selection={'reasoning':'current'},assignment={'started_at_unix_ms':1000,'budget':budget})
        self.assertEqual(g.cold_budget(record,1000001),dict(budget,max_elapsed_seconds=1000))
        self.assertEqual(record['assignment']['started_at_unix_ms'],1000)
        self.assertEqual(budget['max_elapsed_seconds'],900)
        self.assertEqual(g.cold_budget(record,2000),budget)
        record['selection']['reasoning']='managed'
        with self.assertRaises(AssertionError):g.cold_budget(record,1000001)

    def test_input_budget_namespace_is_stable_and_separate_per_batch(self):
        first = Path(os.environ['TMPDIR']) / 'g-first'
        second = Path(os.environ['TMPDIR']) / 'g-second'
        self.assertEqual(g.input_client_id(first), g.input_client_id(first))
        self.assertNotEqual(g.input_client_id(first), g.input_client_id(second))

    def test_remaining_cases_reopen_source_without_replaying_mcp(self):
        events = []

        class Client:
            lane = False

            def call(self, action):
                events.append(action)
                if action in ('init', 'new_source'):
                    assert not self.lane
                    self.lane = True
                elif action == 'release_source':
                    assert self.lane
                    self.lane = False
                return {'requests': []}

        class Batch:
            def __init__(self, root):
                self.plan = {'cases': ['managed-local', 'direct', 'general']}
                self.client = Client()
                self.steps = []
                self.control_observations = 0

            def check(self, **kwargs): pass
            def drain(self): pass
            def close(self): pass

            def managed(self, name, task, **kwargs):
                events.append(name)

            def source_task(self, name, *args):
                assert self.client.lane
                events.append(name)

        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as temp:
            root = Path(temp)
            (root/'plan.json').write_text(json.dumps({'cases':['managed-local','direct','general']}))
            for name in ('direct', 'general'):
                (root / ('task-' + name + '.txt')).write_text('isolated task')
            with patch.object(g, 'Batch', Batch), patch.object(g, 'audit', return_value=[]), patch.object(g.time, 'sleep'):
                g.run(root)
            self.assertTrue(json.loads((root / 'result.json').read_text())['automated_gates'])
        self.assertEqual(events, ['init', 'managed-local', 'direct', 'release_source',
                                  'new_source', 'general', 'release_source', 'snapshot', 'dispose'])


if __name__ == '__main__':
    unittest.main()
