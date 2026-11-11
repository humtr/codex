"""Publication permission is independent of accepted source and scheduling."""
import base64
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import release_control as control

class ReleaseIntentTests(unittest.TestCase):
    def intent(self, **changes):
        value={'schema':'codex-release-intent-v1','source_sha':'a'*40,
               'upstream_version':'0.167.1','publication':True}
        value.update(changes)
        return {'content':base64.b64encode(json.dumps(value).encode()).decode()}

    def test_only_exact_source_version_and_immutable_main_authorize(self):
        for value,expected in [(self.intent(),True),(self.intent(source_sha='c'*40),False),
                               (self.intent(upstream_version='0.167.0'),False)]:
            with patch.object(control,'api',return_value=value) as api:
                self.assertEqual(control.release_authorized('a'*40,'b'*40,'0.167.1'),expected)
                api.assert_called_once_with('contents/release-intent-v1?ref='+'b'*40)
        for value in [self.intent(publication=False),self.intent(publication=1),
                      self.intent(source_sha='main'),self.intent(extra=True)]:
            with patch.object(control,'api',return_value=value):
                with self.assertRaises(ValueError):control.release_authorized('a'*40,'b'*40,'0.167.1')

    def test_missing_intent_is_normal_and_unavailable_authority_is_failure(self):
        with patch.object(control,'api',side_effect=control.GitHubFailure(1,'api','HTTP 404')):
            self.assertFalse(control.release_authorized('a'*40,'b'*40,'0.167.1'))
        with patch.object(control,'api',side_effect=control.GitHubFailure(1,'api','HTTP 503')):
            with self.assertRaises(control.GitHubFailure):control.release_authorized('a'*40,'b'*40,'0.167.1')

    def test_scheduled_controller_has_no_mutations_without_intent(self):
        root=Path(__file__).resolve().parents[2];calls=[]
        def api(path,**kwargs):
            calls.append(path)
            self.assertFalse(kwargs)
            if path=='git/ref/heads/rewrite/rust-core':return {'object':{'sha':'a'*40}}
            if path=='git/ref/heads/main':return {'object':{'sha':'b'*40}}
            if path=='contents/release-intent-v1?ref='+'b'*40:raise control.GitHubFailure(1,'api','HTTP 404')
            self.fail('unexpected controller effect '+path)
        with patch.object(control.subprocess,'check_output',return_value='a'*40+'\n'), \
             patch.object(control,'api',side_effect=api),patch.object(control,'ensure_frontend') as frontend, \
             patch.object(control,'dispatch') as dispatch,patch.object(control,'commit_files') as commit:
            result=control.run(root)
            self.assertEqual(result['state'],'awaiting-release-authorization')
            self.assertEqual(result['source'],'a'*40)
            frontend.assert_not_called();dispatch.assert_not_called();commit.assert_not_called()
            self.assertEqual(len(calls),3)

if __name__=='__main__':unittest.main()
