import subprocess
import sys
from pathlib import Path
import unittest

SCRIPT=Path(__file__).with_name('release_decision.py')


class ReleaseDecisionTests(unittest.TestCase):
    def run_decision(self,**changes):
        args={'current':'0.161.0','upstream':'0.161.0','qualified':'0.161.0','archive':'a'*64,
              'qualified-archive':'a'*64,'source':'b'*40,'sequence':'44','event':'workflow_dispatch',
              'ref':'refs/heads/main','publish':'false','rebuild-current':'false',**changes}
        return subprocess.run([sys.executable,str(SCRIPT),*[v for k,x in args.items() for v in ('--'+k,x)]],text=True,capture_output=True)

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


if __name__=='__main__':unittest.main()
