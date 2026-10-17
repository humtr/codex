import subprocess
import sys
from pathlib import Path
import unittest

SCRIPT=Path(__file__).with_name('release_decision.py')


class ReleaseDecisionTests(unittest.TestCase):
    def test_public_base_is_independent_of_frontend_but_not_of_caller_authority(self):
        base={'variant':'base','qualified':None,'qualified-archive':None,'upstream':'0.164.0'}
        accepted=self.run_decision(**base)
        self.assertEqual(accepted.returncode,0,accepted.stderr)
        self.assertIn('candidate=true',accepted.stdout)
        self.assertIn('generation_id=local-base-relay-hosted-',accepted.stdout)
        for change in [{'event':'push'},{'ref':'refs/heads/rewrite/rust-core'},
                       {'event':'schedule'},{'rebuild-current':'true'},
                       {'base-only':'true','publish':'true'}]:
            result=self.run_decision(**{**base,**change})
            self.assertNotEqual(result.returncode,0,result.stdout)
        result=self.run_decision(**{**base,'event':'schedule','publish':'true'})
        self.assertEqual(result.returncode,0,result.stderr)

    def run_decision(self,**changes):
        args={'current':'0.161.0','upstream':'0.161.0','qualified':'0.161.0','archive':'a'*64,
              'qualified-archive':'a'*64,'source':'b'*40,'sequence':'44','event':'workflow_dispatch',
              'ref':'refs/heads/main','publish':'false','rebuild-current':'false',**changes}
        return subprocess.run([sys.executable,str(SCRIPT),*[v for k,x in args.items() if x is not None for v in ('--'+k,x)]],text=True,capture_output=True)

    def test_real_decision_matrix(self):
        cases=[({},False),({'rebuild-current':'true'},True),
               ({'event':'schedule','publish':'true'},False),
               ({'current':'0.160.0','event':'schedule','publish':'true'},True),
               ({'upstream':'0.162.0'},False),({'current':'0.162.0'},False),
               ({'archive':'c'*64,'rebuild-current':'true'},False)]
        for changes,candidate in cases:
            with self.subTest(changes=changes):
                r=self.run_decision(**changes);self.assertEqual(r.returncode,0,r.stderr)
                output=dict(x.split('=',1) for x in r.stdout.splitlines())
                self.assertEqual(output['candidate'],str(candidate).lower())
                self.assertTrue(output['generation_id'].endswith('-s44'))

    def test_ref_event_pair_and_input_refusals(self):
        cases=[{'ref':'refs/heads/rewrite/rust-core'},{'event':'push'},
               {'event':'schedule','publish':'true','rebuild-current':'true'},
               {'current':'0.160.0','rebuild-current':'true'},
               {'upstream':'0.162.0','rebuild-current':'true'},
               {'current':'0.0161.0'},{'archive':'A'*64},{'source':'main'},
               {'sequence':'0'},{'publish':'yes'},{'event':'schedule'}]
        for changes in cases:
            with self.subTest(changes=changes):
                r=self.run_decision(**changes);self.assertNotEqual(r.returncode,0);self.assertEqual(r.stdout,'')


    def test_base_qualification_does_not_depend_on_frontend_pins(self):
        for event in ('push','workflow_dispatch'):
            r=self.run_decision(**{'base-only':'true','ref':'refs/heads/rewrite/rust-core',
                                  'event':event,'rebuild-current':'true',
                                  'qualified':None,'qualified-archive':None})
            self.assertEqual(r.returncode,0,r.stderr)
            output=dict(line.split('=',1) for line in r.stdout.splitlines())
            self.assertEqual(output['candidate'],'true')
            self.assertEqual(output['decision_reason'],'base-qualification')
            self.assertTrue(output['generation_id'].startswith('base-qualification-'))
            self.assertTrue(output['qualified_artifact_name'].startswith('base-qualified-'))

    def test_base_cannot_publish_or_impersonate_a_public_caller(self):
        base={'base-only':'true','ref':'refs/heads/rewrite/rust-core','event':'push'}
        for bad in ({'publish':'true'},{'event':'schedule'},{'ref':'refs/heads/main'},
                    {'source':'main'},{'rebuild-current':'true','upstream':'0.162.0'}):
            r=self.run_decision(**{**base,**bad})
            self.assertNotEqual(r.returncode,0,r.stdout)
            self.assertEqual(r.stdout,'')

    def test_unqualified_newer_version_is_not_reported_as_up_to_date(self):
        for change,reason in (({'upstream':'0.162.0'},'frontend-unqualified'),
                              ({'current':'0.160.0','archive':'c'*64},'archive-unqualified'),
                              ({},'upstream-not-newer')):
            r=self.run_decision(**change);self.assertEqual(r.returncode,0,r.stderr)
            self.assertIn('decision_reason='+reason,r.stdout)


if __name__=='__main__':unittest.main()
