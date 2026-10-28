import copy
from pathlib import Path
import unittest

from upstream_release import load
from upstream_discovery import proposal

ROOT=Path(__file__).resolve().parents[2]


class DiscoveryTests(unittest.TestCase):
    def test_new_official_tuple_clears_optional_frontend_without_changing_core_toolchain(self):
        before=load(ROOT/'release/upstream.json');before['upstream']['version']='0.163.0'
        original=copy.deepcopy(before);calls=[]
        audit={**before['qualification']['lock_audit'],'external_packages_unchanged':1400}
        def resolve(version):calls.append(version);return 'b'*40,'1.96.0',audit
        result=proposal(before,('0.164.0','a'*64),resolve)
        self.assertEqual(before,original)
        self.assertEqual(result['upstream'],{'version':'0.164.0','source':'b'*40,'archive_sha256':'a'*64})
        self.assertIsNone(result['frontend']);self.assertEqual(result['toolchains']['core_rust'],before['toolchains']['core_rust'])
        self.assertEqual(calls,['0.164.0'])
        self.assertEqual(result['qualification']['lock_audit'],audit)
        for version in ['0.163.0','0.162.0']:
            self.assertIsNone(proposal(before,(version,'a'*64),lambda _:self.fail('must not resolve an old tag')))
        for source,toolchain in [('main','1.96.0'),('b'*40,'nightly')]:
            with self.assertRaises(ValueError):proposal(before,('0.164.0','a'*64),lambda _:(source,toolchain,audit))


if __name__=='__main__':unittest.main()
