import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
from scripts.qualify_provider_f import Batch
from scripts.qualify_provider_f_agents import observe, validate_control_arguments, terminal_record, canonical_item, audit_continuation, receipt_assignments


class AgentFixtureTests(unittest.TestCase):
    def test_receipt_requires_actual_evaluation_dependency_and_settled_tree(self):
        rows=[{'id':n,'phase':'stopped','pending':[],'assignment':{'role':n,'dependencies':[]}} for n in ['receipt-root','receipt-verifier','receipt-dependent']]
        rows[1]['assignment']['evaluation']={'decision':'accepted','findings':'Actual original result verified.'}
        rows[2]['assignment']['dependencies']=['receipt-verifier']
        self.assertEqual(len(receipt_assignments(rows)),3)
        rows[1]['assignment']['evaluation']['findings']=''
        with self.assertRaisesRegex(AssertionError,'not_evaluated'):receipt_assignments(rows)
        rows[1]['assignment']['evaluation']['findings']='Actual original result verified.'
        rows[2]['phase']='stop_requested'
        with self.assertRaisesRegex(AssertionError,'tree_unsettled'):receipt_assignments(rows)

    def test_continuation_requires_original_prefix_and_successful_new_final(self):
        import hashlib
        import subprocess
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);fixture=root/'fixture';fixture.mkdir();home=root/'home';(home/'sessions').mkdir(parents=True)
            names=['agent-coordinator','parallel-writer-a','parallel-writer-b','agent-integrator','independent-reviewer','agent-corrector','correction-verifier','tree-stop-canary']
            records=[]
            for name in names:
                (fixture/name).mkdir()
                records.append({'id':name,'thread':name,'workspace':str(fixture/name),'phase':'stopped','pending':[],
                    'assignment':{'role':name},'execution':{'home':str(home)}})
                rows=[{'type':'event_msg','payload':{'type':'task_complete','last_agent_message':'Verified'}}]*(4 if name=='agent-integrator' else 1)
                (home/'sessions'/(name+'.jsonl')).write_text('\n'.join(map(json.dumps,rows)))
                (root/(name+'-original-cli.txt')).write_text('Verified')
            (fixture/'baseline').mkdir();(fixture/'baseline/test_original.py').write_text('original')
            (fixture/'agent-integrator').rename(fixture/'integrator')
            records[3]['workspace']=str(fixture/'integrator')
            (fixture/'independent-reviewer').rename(fixture/'reviewer');records[4]['workspace']=str(fixture/'reviewer')
            for p in ['baseline/test_original.py','integrator/test_original.py','reviewer/test_review_regression.py','integrator/test_review_regression.py']:
                (fixture/p).write_text('preserved')
            (fixture/'writer-a').mkdir();(fixture/'writer-b').mkdir();(fixture/'writer-b/AFTER_STOP.signal').touch()
            protected=root/'protected';protected.write_bytes(b'original-prefix')
            (root/'prior-proof.json').write_text(json.dumps({'native_prefixes':[{'path':str(protected),'bytes':15,'sha256':hashlib.sha256(protected.read_bytes()).hexdigest()}],'integrator_thread':'agent-integrator'}))
            batch=Mock();batch.plan={'agent_fixture':{'root':str(fixture)}};batch.env={}
            source={'public_messages':['F_AGENTS_CONTINUATION_DONE'],'requests':[{}]}
            with patch('qualify_provider_chat_entry.native_work',return_value=records), \
                    patch('scripts.qualify_provider_f_agents.subprocess.run',return_value=subprocess.CompletedProcess([],0,b'',b'Ran 30 tests')):
                self.assertFalse(audit_continuation(root,batch,source)['full_f_accepted'])
                failed=home/'sessions/correction-verifier.jsonl'
                failed.write_text(json.dumps({'type':'event_msg','payload':{'type':'task_complete','last_agent_message':None,'error':{'message':'stream disconnected'}}}))
                with self.assertRaisesRegex(AssertionError,'continued_native_turn_failed'):
                    audit_continuation(root,batch,source)

    def test_original_rollout_commands_keep_real_failure_evidence(self):
        item=canonical_item({'type':'CommandExecution','exit_code':1,'aggregated_output':'CONFLICT README.md'})
        self.assertEqual((item['type'],item['exitCode'],item['aggregatedOutput']),('commandExecution',1,'CONFLICT README.md'))
        self.assertEqual(canonical_item({'type':'AgentMessage','phase':'final_answer'})['type'],'agentMessage')

    def test_cancelled_original_call_is_preserved_without_becoming_completed_work(self):
        record={'phase':'stopped','pending':['original-offered-call']}
        self.assertTrue(terminal_record(record,True))
        self.assertFalse(terminal_record(record,False))
        self.assertFalse(terminal_record({'phase':'stop_requested','pending':[]},True))

    def test_declared_native_question_is_answered_once_without_replaying_unknown_answer(self):
        with tempfile.TemporaryDirectory() as directory:
            batch=Mock();batch.root=Path(directory);batch.rpc.return_value={'answered':True}
            row={'id':'root','capability':'private','assignment':{'role':'agent-coordinator'}}
            status={'human_requests':[{'id':17,'params':{'questions':[{'id':'f_validation_policy','options':[{'label':'Strict (Recommended)'}]}]}}]}
            observe(batch,row,status);observe(batch,row,status)
            batch.rpc.assert_called_once()
            self.assertEqual(batch.rpc.call_args.args[0],'codex_answer')
            self.assertEqual(set(batch.rpc.call_args.args[1]), {'work_id','controller','request_id','answers'})
            self.assertEqual(batch.rpc.call_args.args[1]['answers'],{'f_validation_policy':{'answers':['Strict (Recommended)']}})

    def test_actual_declared_argument_boundary_is_checked_before_dispatch(self):
        tools=[{'name':'codex_answer','inputSchema':{'properties':dict.fromkeys(['work_id','controller','request_id','answers']),
            'required':['work_id','controller','request_id','answers']}}]
        args={'work_id':'w','controller':'c','request_id':'0','answers':{}}
        validate_control_arguments(tools,'codex_answer',args)
        with self.assertRaisesRegex(AssertionError,'fixture_unknown_control_argument'):
            validate_control_arguments(tools,'codex_answer',{**args,'call_id':'unsupported'})

    def test_unexpected_question_never_gets_an_automatic_answer(self):
        batch=Mock();row={'assignment':{'role':'agent-coordinator'}}
        with self.assertRaisesRegex(AssertionError,'unexpected_fixture_question'):
            observe(batch,row,{'human_requests':[{'params':{'questions':[{'id':'real_user_decision'}]}}]})
        batch.rpc.assert_not_called()

    def test_declared_stop_uses_two_real_markers_and_releases_only_the_owned_sibling_once(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);fixture=root/'fixture';(fixture/'writer-a').mkdir(parents=True);(fixture/'writer-b').mkdir()
            batch=Mock();batch.root=root;batch.plan={'agent_fixture':{'root':str(fixture)}};batch.expected_stops=set()
            batch.rpc.side_effect=[{'completed':False},{'native_stop_confirmed':True,'host_stop_confirmed':True,'remaining_jobs':[]}]
            a={'id':'a','assignment':{'role':'parallel-writer-a'}}
            b={'id':'b','workspace':str(fixture/'writer-b'),'phase':'running','assignment':{'role':'parallel-writer-b'}}
            observe(batch,a,{})
            batch.rpc.assert_not_called()
            (fixture/'writer-a/READY.signal').touch();(fixture/'writer-b/BEFORE_STOP.signal').touch()
            with patch('qualify_provider_chat_entry.native_work',return_value=[a,b]):
                observe(batch,a,{});observe(batch,a,{})
            self.assertEqual(batch.rpc.call_count,2)
            self.assertEqual(batch.rpc.call_args.args,('codex_stop',{'work_id':'a'}))
            self.assertTrue((fixture/'writer-b/RELEASE.signal').exists())
            self.assertEqual(batch.expected_stops,{'a'})
            self.assertTrue(json.loads((root/'declared-writer-stop-intent.json').read_text())['both_native_workspaces_observed_active'])
