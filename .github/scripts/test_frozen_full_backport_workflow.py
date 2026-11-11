#!/usr/bin/env python3
"""Scope a frozen Full signer to previously proven c19 source and signed S161 Base."""
from pathlib import Path
import unittest
import yaml

ROOT=Path(__file__).resolve().parents[2]
FULL=(ROOT/'.github/workflows/auto-release-termux.yml').read_text()
QUAL=(ROOT/'.github/workflows/qualify-termux-runtime.yml').read_text()
SOURCE='c19b61bb3d1d3bdb7e3c4f2ad9763d71397e5ca0'
BASE='core-v0.167.0-c19b61b-s161'
HISTORY='8bb6c6e6a0eab37ef12958f83eb918c14e2c2d33'

class FrozenFullContract(unittest.TestCase):
    def test_reusable_workflow_wiring_is_explicit_and_optional(self):
        for content in (FULL,QUAL):
            doc=yaml.load(content,Loader=yaml.BaseLoader)
            definition=doc['on']['workflow_call']['inputs']['historical_base_generation']
            self.assertEqual(definition['default'],'')
            self.assertEqual(definition['type'],'string')
            self.assertIn("test '${{ inputs.historical_base_generation }}' = '"+BASE+"'",content)
            self.assertIn("test \"$CODEX_SOURCE_SHA\" = '"+SOURCE+"'",content)
            self.assertIn('if test -n \'${{ inputs.historical_base_generation }}\'; then',content)
        self.assertIn('historical_base_generation: ${{ inputs.historical_base_generation }}',FULL)
        self.assertIn('release_variant: ${{ inputs.release_variant }}',FULL)
        self.assertIn('publish: ${{ inputs.publish }}',FULL)
    def test_historical_base_is_authenticated_at_every_consumption(self):
        for workflow,count in ((FULL,2),(QUAL,1)):
            self.assertEqual(workflow.count("--main-sha '"+HISTORY+"' --variant base"),count)
            self.assertEqual(workflow.count("grep -Fqx 'current_generation="+BASE+"'"),count)
            self.assertEqual(workflow.count('and .heads.full.generation == "full-v0.167.0-681ecce-s159"'),count)
            self.assertEqual(workflow.count('"core-v0.167.1-f9dfc3f-s162"'),count)
        self.assertIn('python3 source/.github/scripts/release_components.py acquire-base',QUAL)
        self.assertIn('python3 source/.github/scripts/release_components.py match-base',FULL)
        self.assertIn('python3 publication-source/.github/scripts/release_components.py match-base',FULL)
    def test_live_signed_head_controls_sequence_and_promotion(self):
        self.assertIn('release_sequence: ${{ steps.stable.outputs.release_sequence }}',QUAL)
        self.assertIn('expected_main_sha: ${{ steps.stable.outputs.expected_main_sha }}',QUAL)
        self.assertIn('current_generation: ${{ steps.stable.outputs.current_generation }}',QUAL)
        self.assertIn('--main-sha "$expected_main_sha" --variant',QUAL)
        self.assertIn('expected_main=\'${{ needs.producer.outputs.expected_main_sha }}\'',FULL)
        self.assertIn('test "$(gh api "repos/$GITHUB_REPOSITORY/git/ref/heads/main" --jq \'.object.sha\')" = "$expected_main"',FULL)
        self.assertIn('CODEX_RELEASE_SIGNING_KEY: ${{ secrets.CODEX_RELEASE_SIGNING_KEY }}',FULL)
        self.assertIn('openssl pkeyutl -verify -pubin -inkey "$authority" -rawin',FULL)

if __name__=='__main__':unittest.main()
