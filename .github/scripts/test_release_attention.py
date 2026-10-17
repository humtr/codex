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


if __name__=='__main__':unittest.main()
