"""Protect release privilege boundaries and parse every executable shell step."""
from pathlib import Path
import re
import subprocess
import sys
import unittest

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts'))
from pin_release import caller


class WorkflowTests(unittest.TestCase):
    def test_action_pins_and_shell_syntax(self):
        for p in (ROOT/'.github/workflows').glob('*.yml'):
            text=p.read_text()
            for use in re.findall(r'(?m)^\s*uses:\s*(\S+)',text):
                if not use.startswith('./'):self.assertRegex(use,r'@[0-9a-f]{40}$')
            # All workflows use literal run blocks. Extract by next indentation.
            for m in re.finditer(r'(?m)^( +)run: \|\n',text):
                indent=len(m[1]);lines=[]
                for line in text[m.end():].splitlines():
                    if line.strip() and len(line)-len(line.lstrip())<=indent:break
                    lines.append(line[indent+2:])
                script=re.sub(r'\$\{\{.*?\}\}','placeholder','\n'.join(lines))
                result=subprocess.run(['bash','-n'],input=script,text=True,capture_output=True)
                self.assertEqual(result.returncode,0,str(p)+': '+result.stderr)

    def test_single_source_and_no_unsigned_public_authority(self):
        text=(ROOT/'.github/workflows/auto-release-termux.yml').read_text()
        header,body=text.split('\njobs:\n',1)
        producer=body.split('\n  smoke:',1)[0]
        smoke=body.split('\n  smoke:',1)[1].split('\n  sign:',1)[0]
        sign=body.split('\n  sign:',1)[1].split('\n  stage_release:',1)[0]
        self.assertIn('workflow_call:',header)
        self.assertNotIn('workflow_dispatch:',header)
        self.assertIn('CODEX_SOURCE_SHA: ${{ inputs.source_sha }}',header)
        self.assertNotIn('RALD5_SOURCE_SHA',text)
        self.assertIn('publication_source_sha: ${{ inputs.source_sha }}',text)
        for job in (producer,smoke,sign):
            for forbidden in ('contents: write','pages: write','gh release','git push','deploy-pages'):
                self.assertNotIn(forbidden,job)
        self.assertNotIn('secrets.',producer+smoke)
        self.assertEqual(text.count('${{ secrets.CODEX_RELEASE_SIGNING_KEY }}'),1)
        self.assertIn("needs.smoke.result == 'success' && inputs.publish",sign)
        for key in ('stage_release','pages','verify_public','promote'):
            job=re.split(r'\n  [a-z_]+:',body.split('\n  '+key+':',1)[1],maxsplit=1)[0]
            self.assertIn('&& inputs.publish',job)
        self.assertIn('force:false',text)
        self.assertNotIn('force:true',text)
        self.assertIn('Bootstrap protected source state and update through staged Pages',text)
        self.assertIn('Prepare manifest-owned current generation bootstrap fixture',text)
        self.assertIn('rald5-after-noop',text)
        self.assertNotIn('same_version_acceptance',text)
        self.assertNotIn('legacy_lag_jump_remediation',text)

    def test_main_caller_pins_both_inputs_and_default_dry_run(self):
        sha='a'*40; text=caller(sha)
        self.assertIn('/auto-release-termux.yml@'+sha,text)
        self.assertIn('source_sha: '+sha,text)
        self.assertEqual(text.count('default: false'),2)
        self.assertEqual(text.count('secrets.CODEX_RELEASE_SIGNING_KEY'),1)
        self.assertNotIn('secrets: inherit',text)
        for bad in ('main','a'*39,'A'*40,'a'*40+'\ninjected'):
            with self.assertRaises(ValueError):caller(bad)


if __name__=='__main__':unittest.main()
