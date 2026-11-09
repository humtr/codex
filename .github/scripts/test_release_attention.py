import base64
import unittest
from unittest.mock import patch

import release_attention as attention


class AttentionTests(unittest.TestCase):
    def fixture(self):
        return {'id':123,'path':'.github/workflows/auto-release-termux.yml','head_sha':'a'*40,
                'head_branch':'main','event':'workflow_dispatch','status':'completed','conclusion':'failure',
                'repository':{'full_name':'humtr/codex'}}

    def test_only_owned_terminal_unexpected_failures_call_owner(self):
        run=self.fixture();jobs=[{'name':'full / Android proof','conclusion':'failure'}]
        value=attention.classify(run,jobs);self.assertIsNotNone(value)
        for key,bad in [('head_branch','other'),('event','pull_request'),('status','in_progress'),
                        ('repository',{'full_name':'other/repo'}),('conclusion','success'),('head_sha','main')]:
            self.assertIsNone(attention.classify({**run,key:bad},jobs))
        # A passing job can contain expected negative tests, never an alert.
        self.assertIsNone(attention.classify({**run,'conclusion':'success'},[{'name':'negative case','conclusion':'success'}]))

    def test_repeated_failure_does_not_create_another_issue_or_notification(self):
        run=self.fixture();jobs=[{'name':'full / Android proof','conclusion':'failure'}]
        workflow,_,digest=attention.classify(run,jobs)
        body='<!-- codex-release-attention:'+workflow+' -->\n<!-- failure:'+digest+' -->'
        def api(path,**kwargs):
            if path=='actions/runs/123':return run
            if path.endswith('/jobs?per_page=100'):return {'jobs':jobs}
            if path.startswith('contents/'):raise KeyError('owned missing fixture')
            if path.startswith('issues?'):return [{'number':5,'body':body}]
            self.fail('unexpected write or lookup: '+path)
        with patch.object(attention,'api',side_effect=api):
            self.assertEqual(attention.report(123),{'state':'already-notified','issue':5})

    def test_failure_notice_retains_current_and_historical_generation_identity(self):
        for base, full in [('core-v0.155.0-s12','full-v0.155.0-s13'),
                           ('local-base-proof-12','local-full-proof-13')]:
            writes=[]
            def api(path,**kwargs):
                if path=='actions/runs/123':return self.fixture()
                if path.endswith('/jobs?per_page=100'):
                    return {'jobs':[{'name':'full / Android proof','conclusion':'failure'}]}
                if path.startswith('contents/'):
                    value=base if 'update-index-base-v1' in path else full
                    text=('codex-update-index-v1\nchannel\tstable\ngeneration_id\t'+value+
                          '\nrelease_base\thttps://example.invalid/'+value+'/\n')
                    return {'content':base64.b64encode(text.encode()).decode()}
                if path.startswith('issues?'):return []
                if path.startswith('labels?'):return [{'name':attention.LABEL}]
                if path=='issues':
                    writes.append(kwargs['payload']);return {'number':7}
                self.fail('unexpected request: '+path)
            with self.subTest(base=base), patch.object(attention,'api',side_effect=api):
                self.assertEqual(attention.report(123),{'state':'attention-created','issue':7})
                self.assertEqual(len(writes),1)
                self.assertIn('repair target: `'+base+'`',writes[0]['body'])
                self.assertIn('fallback: `'+full+'`',writes[0]['body'])


if __name__=='__main__':unittest.main()
