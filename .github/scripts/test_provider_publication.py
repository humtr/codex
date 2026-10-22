import hashlib
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('provider_publication', Path(__file__).with_name('provider_publication.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class ProviderPublicationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='provider-publication-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.package = self.root / 'package'
        (self.package / 'licenses').mkdir(parents=True)
        self.openssl = Path(shutil.which('openssl'))
        self.private = self.root / 'private.pem'
        self.public = self.root / 'public.pem'
        subprocess.run([self.openssl, 'genpkey', '-algorithm', 'Ed25519', '-out', self.private], check=True, capture_output=True)
        subprocess.run([self.openssl, 'pkey', '-in', self.private, '-pubout', '-out', self.public], check=True, capture_output=True)
        entries = []
        for name in p.FILES:
            data = bytearray(b'x' * 64)
            if name == 'codex-chatgpt-provider':
                data[:6] = b'\x7fELF\x02\x01'
                data[18:20] = b'\xb7\0'
            target = self.package / name
            target.write_bytes(data)
            mode = 0o755 if name == 'codex-chatgpt-provider' else 0o644
            target.chmod(mode)
            entries.append(f'file\t{name}\t{hashlib.sha256(data).hexdigest()}\t64\t{mode:04o}')
        self.manifest = ('codex-provider-payload-v1\nprovider\tchatgpt\ngeneration_id\tchatgpt-test-1\nrelease_sequence\t1\n'
                         'source\t' + 'a' * 40 + '\nsource_inputs_sha256\t' + hashlib.sha256(b'x' * 64).hexdigest() +
                         '\ntarget\taarch64-linux-android\ncore_api\tprovider-payload-v1\ndependencies\t' + p.DEPENDENCIES +
                         '\nfile_count\t8\n' + '\n'.join(entries) + '\n')
        (self.package / 'payload.manifest').write_text(self.manifest)
        subprocess.run([self.openssl, 'pkeyutl', '-sign', '-rawin', '-inkey', self.private,
                        '-in', self.package / 'payload.manifest', '-out', self.package / 'payload.sig'], check=True, capture_output=True)

    def test_signature_inventory_export_restore(self):
        self.assertEqual(p.verify(self.package, self.public, self.openssl)['generation_id'], 'chatgpt-test-1')
        assets = self.root / 'assets'
        p.export(self.package, assets, self.public, self.openssl)
        self.assertEqual(len(list(assets.iterdir())), 10)
        restored = self.root / 'restored'
        p.restore(assets, restored, self.public, self.openssl)
        self.assertEqual(p.verify(restored, self.public, self.openssl)['release_sequence'], '1')
        (assets / 'extra').write_text('extra')
        with self.assertRaises(ValueError):
            p.restore(assets, self.root / 'extra-restore', self.public, self.openssl)

    def test_unknown_authority_and_changed_signature_fail(self):
        other_private, other_public = self.root / 'other.pem', self.root / 'other-public.pem'
        subprocess.run([self.openssl, 'genpkey', '-algorithm', 'Ed25519', '-out', other_private], check=True, capture_output=True)
        subprocess.run([self.openssl, 'pkey', '-in', other_private, '-pubout', '-out', other_public], check=True, capture_output=True)
        with self.assertRaises(subprocess.CalledProcessError):
            p.verify(self.package, other_public, self.openssl)
        (self.package / 'payload.sig').write_bytes(b'x')
        with self.assertRaises(ValueError):
            p.verify(self.package, self.public, self.openssl)

    def test_tamper_mode_symlink_and_extra_reject(self):
        file = self.package / 'NOTICE.md'
        file.write_bytes(b'z' * 64)
        with self.assertRaises(ValueError):
            p.verify(self.package, self.public, self.openssl)
        file.write_bytes(b'x' * 64)
        file.chmod(0o777)
        with self.assertRaises(ValueError):
            p.verify(self.package, self.public, self.openssl)
        file.unlink()
        file.symlink_to(self.package / 'Cargo.lock')
        with self.assertRaises(ValueError):
            p.verify(self.package, self.public, self.openssl)

    def test_manifest_canonical_order_and_bounds(self):
        for invalid in (self.manifest.replace('\n', '\r\n'), self.manifest + '\n',
                        self.manifest.replace('release_sequence\t1', 'release_sequence\t01'),
                        self.manifest.replace('chatgpt-test-1', '../escape'),
                        self.manifest.replace('file\tNOTICE.md', 'file\tunknown'),
                        self.manifest.replace('\t64\t0644', '\t1048577\t0644')):
            with self.subTest(invalid=invalid[:80]), self.assertRaises(ValueError):
                p.parse(invalid.encode())
        with self.assertRaises(ValueError):
            p.readback('http://other.example/', self.root / 'download', self.public, self.openssl, Path(shutil.which('curl')))

    def signed_index(self):
        index=self.root/'provider-index-v1'
        index.write_text('codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\ngeneration_id\tchatgpt-test-1\nrelease_sequence\t1\nrelease_base\thttps://github.com/humtr/codex/releases/download/chatgpt-test-1/\nmanifest_sha256\t'+hashlib.sha256((self.package/'payload.manifest').read_bytes()).hexdigest()+'\n')
        signature=self.root/'provider-index-v1.sig'
        subprocess.run([self.openssl,'pkeyutl','-sign','-rawin','-inkey',self.private,'-in',index,'-out',signature],check=True,capture_output=True)
        return index,signature

    def test_stable_index_exact_binding(self):
        index,signature=self.signed_index()
        p.verify_index(index,signature,self.package,self.public,self.openssl)
        original=index.read_bytes()
        for bad in [original.replace(b'channel\tstable',b'channel\tother'),original.replace(b'chatgpt-test-1',b'../escape'),original.replace(b'https://github.com/humtr/codex/',b'https://foreign.example/'),original.replace(b'\n',b'\r\n'),original+b'\n']:
            with self.assertRaises(ValueError):p.parse_index(bad)
        index.write_bytes(original.replace(b'release_sequence\t1',b'release_sequence\t2'))
        with self.assertRaises((ValueError,subprocess.CalledProcessError)):
            p.verify_index(index,signature,self.package,self.public,self.openssl)

    def test_promotion_requires_consumer_and_edits_only_independent_pointer(self):
        from types import SimpleNamespace
        from unittest.mock import patch
        import base64
        import json
        promotion_spec=importlib.util.spec_from_file_location('provider_promotion',Path(__file__).with_name('provider_promotion.py'))
        import sys
        sys.modules['provider_publication']=p
        promotion=importlib.util.module_from_spec(promotion_spec);promotion_spec.loader.exec_module(promotion)
        index,signature=self.signed_index()
        proof=self.root/'consumer.json'
        record=dict(schema='termux-codex-provider-public-consumer-v1',accepted=True,generation='chatgpt-test-1',source_sha='a'*40,manifest_sha256=hashlib.sha256((self.package/'payload.manifest').read_bytes()).hexdigest())
        record.update({name:True for name in ('complete_public_readback','actual_android','disposable_install','dispatch','rollback','native_defaults_preserved','operating_runtime_unchanged')})
        proof.write_text(json.dumps(record))
        args=SimpleNamespace(index=index,signature=signature,package=self.package,public_key=self.public,proof=proof,openssl=self.openssl,expected_main='b'*40,apply=False)
        with patch.object(promotion,'api',side_effect=AssertionError('dry run must not call network')):
            result=promotion.promote(args);self.assertFalse(result['mutation'])
            self.assertEqual(result['paths'],['provider-index-v1','provider-index-v1.sig','provider-source-v1'])
        args.apply=True
        calls=[]
        blobs={}
        promoted_entries=[]
        outputs={'provider-index-v1':index.read_bytes(),'provider-index-v1.sig':signature.read_bytes(),'provider-source-v1':b'a'*40+b'\n'}
        def fake_api(path,method='GET',body=None):
            calls.append((path,method,body))
            if path=='git/ref/heads/main':return {'object':{'sha':args.expected_main if sum(v[0]==path for v in calls)==1 else 'c'*40}}
            if path=='git/commits/'+'a'*40:return {'sha':'a'*40}
            if path=='git/commits/'+args.expected_main:return {'tree':{'sha':'tree-before'}}
            if path=='git/trees/tree-before':return {'tree':[]}
            if path=='git/blobs':
                identity='blob-'+str(len(calls));blobs[identity]=body['content'];return {'sha':identity}
            if path=='git/trees':
                self.assertEqual({v['path'] for v in body['tree']},set(outputs));promoted_entries.extend(body['tree']);return {'sha':'tree-after'}
            if path=='git/trees/tree-after':return {'tree':promoted_entries}
            if path.startswith('git/blobs/'):return {'content':blobs[path.rsplit('/',1)[1]],'encoding':'base64'}
            if path=='git/commits':return {'sha':'c'*40}
            if path=='git/refs/heads/main':self.assertEqual(body,{'sha':'c'*40,'force':False});return {}
            if path.startswith('contents/'):
                raise AssertionError('contents API text conversion is not binary signature readback')
            raise AssertionError(path)
        with patch.object(promotion,'api',side_effect=fake_api):
            self.assertTrue(promotion.promote(args)['promoted'])
        self.assertEqual(sum(method=='PATCH' for _,method,_ in calls),1)
        record['accepted']=False;proof.write_text(json.dumps(record))
        with patch.object(promotion,'api',side_effect=AssertionError('invalid proof must not mutate')):
            with self.assertRaises(ValueError):promotion.promote(args)


if __name__ == '__main__':
    unittest.main()
