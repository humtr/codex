import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from io import BytesIO
from urllib.error import HTTPError

import release_heads as channel
from rald3_preflight import PACKAGE_IDENTITY
from rald5_publication import BASE_FILES, EXPECTED_FILES, EXTENDED_FILES, ValidationError


class ReleaseHeadsTests(unittest.TestCase):
    def test_public_transport_identifies_client_and_preserves_failure_bounds(self):
        url='https://codex-releases.humtr.workers.dev/local-base-relay-hosted-test/release.manifest'
        with tempfile.TemporaryDirectory() as tmp:
            target=Path(tmp)/'result'
            response=BytesIO(b'owned')
            response.geturl=lambda:url
            with patch.object(channel,'urlopen',return_value=response) as fetch:
                self.assertTrue(channel.download(url,target,5))
                request=fetch.call_args.args[0]
                self.assertEqual(request.full_url,url)
                self.assertEqual(request.get_header('User-agent'),'codex-release-qualification/1')
                self.assertEqual(target.read_bytes(),b'owned')
            target.unlink()
            for status in [403,404]:
                error=HTTPError(url,status,'unavailable',{},None)
                with patch.object(channel,'urlopen',side_effect=error):
                    if status==404:self.assertFalse(channel.download(url,target,5,optional=True))
                    else:
                        with self.assertRaisesRegex(ValueError,'HTTP 403'):
                            channel.download(url,target,5,optional=True)
                self.assertFalse(target.exists())
            for returned,data,message in [(url,b'oversized','exceeds bound'),
                    ('http://insecure.invalid/',b'owned','left HTTPS')]:
                response=BytesIO(data);response.geturl=lambda:returned
                with patch.object(channel,'urlopen',return_value=response):
                    with self.assertRaisesRegex(ValueError,message):channel.download(url,target,5)
                self.assertFalse(target.exists())

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='release-heads-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.key, self.public = self.root/'private.pem', self.root/'public.pem'
        self.openssl('genpkey', '-algorithm', 'ED25519', '-out', str(self.key))
        self.openssl('pkey', '-in', str(self.key), '-pubout', '-out', str(self.public))
        der = subprocess.check_output(['openssl', 'pkey', '-pubin', '-in', str(self.public), '-outform', 'DER'], stderr=subprocess.DEVNULL)
        self.raw = der[-32:].hex()
        self.objects = {}
        self.main = 'a'*40
        self.head('full', 'local-full-proof-11', 11)

    def openssl(self, *args):
        subprocess.run(['openssl', *args], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def signature(self, value):
        data, sig = self.root/'sign-input', self.root/'sign-output'
        data.write_bytes(value)
        self.openssl('pkeyutl', '-sign', '-rawin', '-inkey', str(self.key), '-in', str(data), '-out', str(sig))
        return sig.read_bytes()

    def head(self, lane, generation, sequence, frontend=False):
        inventory = BASE_FILES if lane == 'base' else EXTENDED_FILES if frontend else EXPECTED_FILES
        assets = {name: ('owned:'+name).encode() for name in inventory}
        digest = lambda name: hashlib.sha256(assets[name]).hexdigest()
        assets['generation.meta'] = ('\n'.join([
            'codex-local-generation-v2', 'generation_id\t'+generation,
            'upstream_package_identity\t'+PACKAGE_IDENTITY, 'upstream_package_version\t0.155.0',
            'source_artifact_digest\t'+'b'*64, 'expected_platform\tandroid', 'expected_architecture\taarch64',
            'patch_policy_id\ttermux-fd-remap-v1', 'patch_report\tfixture',
            'runtime_digest\t'+digest('runtime'), 'core_artifact_digest\t'+digest('core'),
            'manager_artifact_digest\t'+(digest('manager') if lane == 'full' else '-'),
            'core_api_identity\tcore-api-v1', 'persistent_schema_identity\tschema-v1',
            'qualification\tqualified', 'creation_metadata\tr10-browser-helper-bridge-v1',
            'upstream_doctor\tunsupported', 'helper_count\t'+('3' if frontend else '2'),
            'helper\ttermux-browser-open-v1\t'+digest('helpers/0'),
            'helper\ttermux-browser-manual-v1\t'+digest('helpers/1'),
            *(['helper\ttermux-manager-tui-v1:0.155.0\t'+digest('helpers/2')] if frontend else []), '',
        ])).encode()
        manifest = ('\n'.join([
            'codex-release-v4', 'generation_id\t'+generation, 'release_sequence\t'+str(sequence),
            'channel\tstable', 'expected_platform\tandroid', 'expected_architecture\taarch64',
            'core_api_identity\tcore-api-v1', 'persistent_schema_identity\tschema-v1',
            'release_public_key\t'+self.raw, 'file_count\t'+str(len(inventory)),
            *['file\t'+name+'\t'+digest(name)+'\t'+mode for name, mode in inventory.items()], '',
        ])).encode()
        base = channel.canonical_release_base(generation)
        index = ('codex-update-index-v1\nchannel\tstable\ngeneration_id\t'+generation+'\nrelease_base\t'+base+'\n').encode()
        url = f'https://raw.githubusercontent.com/humtr/codex/{self.main}/{channel.HEADS[lane]}'
        self.objects.update({url:index, url+'.sig':self.signature(index), base+'release.manifest':manifest,
                             base+'release.sig':self.signature(manifest), **{base+name:data for name,data in assets.items()}})

    def fetch(self, url, target, limit, *, optional=False):
        if optional and url not in self.objects:return False
        data = self.objects[url];self.assertLessEqual(len(data), limit)
        target.parent.mkdir(parents=True, exist_ok=True);target.write_bytes(data);return True

    def resolve(self, variant, name='heads'):
        return channel.authenticate(self.main, variant, self.public, self.root/name, fetch=self.fetch)

    def test_two_authenticated_heads_share_global_sequence_but_keep_selection(self):
        self.head('base', 'local-base-proof-40', 40)
        heads, result = self.resolve('full')
        self.assertEqual(result['release_sequence'], '41')
        self.assertEqual(result['current_generation'], 'local-full-proof-11')
        self.assertEqual(len(result['retained_generations']), 2)
        self.assertEqual(channel.plan(heads, 'base')['current_generation'], 'local-base-proof-40')
        self.assertEqual(result['bootstrap_base'], 'false')

    def test_first_base_uses_authenticated_full_as_seed_only(self):
        _, result = self.resolve('base')
        self.assertEqual(result['bootstrap_base'], 'true')
        self.assertEqual(result['current_index_path'], 'update-index-v1')
        self.assertEqual(result['release_sequence'], '12')

    def test_invalid_signature_or_descriptor_cannot_supply_release_sequence(self):
        url = f'https://raw.githubusercontent.com/humtr/codex/{self.main}/update-index-v1.sig'
        self.objects[url] = b'0'*64
        with self.assertRaises(ValidationError):self.resolve('full')
        self.head('full', 'local-full-proof-11', 11)
        self.objects[channel.canonical_release_base('local-full-proof-11')+'generation.meta'] += b'tamper'
        with self.assertRaisesRegex(ValueError, 'signed inventory'):self.resolve('full', 'other')

    def test_pages_restoration_keeps_both_signed_heads_and_rejects_corrupt_assets(self):
        self.head('base', 'local-base-proof-40', 40)
        heads, _ = self.resolve('base')
        site = self.root/'site';site.mkdir()
        channel.restore(self.root/'heads', heads, self.public, site, fetch=self.fetch)
        self.assertEqual({p.name for p in site.iterdir()}, {'local-full-proof-11','local-base-proof-40'})
        self.assertFalse((site/'local-base-proof-40/manager').exists())
        self.assertTrue((site/'local-full-proof-11/manager').is_file())
        # A second pass verifies pre-existing LKG rather than blindly trusting it.
        (site/'local-full-proof-11/core').write_bytes(b'corrupt')
        with self.assertRaises(ValidationError):
            channel.restore(self.root/'heads', heads, self.public, site, fetch=self.fetch)


if __name__ == '__main__':unittest.main()
