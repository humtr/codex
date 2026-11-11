"""Original Full input IDs govern transfer, independent of later named runs."""
import copy
import hashlib
import io
import json
from pathlib import Path
import stat
import tempfile
import unittest
import warnings
from unittest.mock import patch
import zipfile

import full_qualification as full
from runtime_inputs import canonical
import test_full_qualification as fixture_module

ROOT=fixture_module.ROOT
SHA=fixture_module.SHA
from tui_artifact import FILES


class FullInputAcquisitionTests(unittest.TestCase):
    def setUp(self):
        fixtures=fixture_module.FullQualificationTests()
        self.plan=fixtures.resolve()
        for number,entry in enumerate(self.plan['artifacts'].values(),101):entry['id']=number
        self.temporary=tempfile.TemporaryDirectory();self.addCleanup(self.temporary.cleanup)
        self.root=Path(self.temporary.name)
        mocked=patch.object(full,'git',side_effect=fixtures.source_git);mocked.start();self.addCleanup(mocked.stop)
        self.archives={};self.rows={}
        for key,entry in self.plan['artifacts'].items():
            data=self.zip(self.files(key));self.archives[entry['id']]=data
            self.rows[entry['id']]={'id':entry['id'],'name':entry['name'],'expired':False,
                'size_in_bytes':len(data),'digest':'sha256:'+hashlib.sha256(data).hexdigest(),
                'workflow_run':{'id':entry['run_id'],'head_sha':SHA}}

    def files(self,key):
        if key in ('android-core','android-manager'):return {'component.tar':b'owned immutable component transport'}
        if key=='core-candidate':return {'qualified-candidate.tar':b'owned immutable Runtime transport'}
        if key=='manager-proof':return {'manager-qualification.json':b'{}'}
        if key=='tui':return {name:('owned '+name).encode() for name in FILES}
        return {'_temp/profile-tui-focused.log':b'owned exact-run execution log',
                'codex/upstream/snapshots/profile_manager.snap':b'owned snapshot'}

    def zip(self,files,*,symlink=None,duplicate=None,alias=False):
        output=io.BytesIO()
        with zipfile.ZipFile(output,'w',compression=zipfile.ZIP_DEFLATED) as archive:
            for name,data in files.items():
                info=zipfile.ZipInfo(name);info.compress_type=zipfile.ZIP_DEFLATED
                info.external_attr=((stat.S_IFLNK if name==symlink else stat.S_IFREG)|0o644)<<16
                archive.writestr(info,data)
            if duplicate:
                with warnings.catch_warnings():
                    warnings.simplefilter('ignore',UserWarning);archive.writestr(duplicate,files[duplicate])
            if alias:
                name=next(iter(files))+'/'
                info=zipfile.ZipInfo(name);info.external_attr=(stat.S_IFDIR|0o700)<<16;archive.writestr(info,b'')
        return output.getvalue()

    def acquire(self,*,change=None,fetch_change=None,plan=None):
        self.calls=[];self.fetched=[]
        def request(path):
            self.calls.append(path)
            if path.startswith('actions/artifacts/'):
                value=copy.deepcopy(self.rows[int(path.rsplit('/',1)[1])])
            elif path.startswith('actions/runs/'):
                number=int(path.rsplit('/',1)[1])
                value=copy.deepcopy(next(run for run in self.plan['runs'].values() if run['id']==number))
            else:self.fail('name/latest/list lookup must not reinterpret saved input IDs: '+path)
            if change:change(path,value)
            return value
        def fetch(row,path):
            self.fetched.append(row['id']);data=self.archives[row['id']]
            path.write_bytes(fetch_change(data) if fetch_change else data)
        return full.acquire(ROOT,self.root/'inputs',self.plan if plan is None else plan,request,
                            lambda *args:self.fail('saved plan must never select a newer successful run'),fetch)

    def test_saved_plan_consumes_only_original_ids_and_preserves_origin(self):
        result=self.acquire()
        self.assertEqual(result,self.plan)
        saved=json.loads((self.root/'inputs/inputs.json').read_bytes())
        self.assertEqual(saved,self.plan)
        origin=json.loads((self.root/'inputs/artifact-origins.json').read_bytes())
        self.assertEqual(origin['source_sha'],SHA)
        self.assertEqual(set(origin['artifacts']),set(self.plan['artifacts']))
        for key,entry in self.plan['artifacts'].items():
            record=origin['artifacts'][key];row=self.rows[entry['id']]
            self.assertEqual(record,{'id':entry['id'],'name':entry['name'],'run_id':entry['run_id'],
                'source_sha':SHA,'archive_digest':row['digest'],'archive_bytes':row['size_in_bytes']})
            for name,data in self.files(key).items():self.assertEqual((self.root/'inputs'/key/name).read_bytes(),data)
            self.assertFalse((self.root/'inputs'/key/'.download.zip').exists())
        self.assertEqual(self.fetched,[entry['id'] for entry in self.plan['artifacts'].values()])
        self.assertEqual(sum(path.startswith('actions/runs/') for path in self.calls),6)

    def test_wrong_id_source_run_name_expiry_or_metadata_never_uses_name_fallback(self):
        mutations=[
            lambda row:row.update(id=row['id']+1000),
            lambda row:row.update(name='same-name-from-another-role'),
            lambda row:row.update(expired=True),
            lambda row:row['workflow_run'].update(head_sha='b'*40),
            lambda row:row['workflow_run'].update(id=999),
            lambda row:row.update(digest='unverified'),
            lambda row:row.update(size_in_bytes=full.LIMIT+1),
        ]
        for number,mutation in enumerate(mutations):
            def change(path,value):
                if path=='actions/artifacts/103':mutation(value)
            with self.subTest(fault=number),self.assertRaises(ValueError):self.acquire(change=change)
            self.assertFalse((self.root/'inputs').exists())
            self.assertNotIn(103,self.fetched)
            self.assertFalse(any('/workflows/' in path or '?' in path for path in self.calls))

    def test_actual_download_bytes_and_bound_safe_zip_are_required(self):
        for change in (lambda data:data+b'changed',lambda data:data[:-1]):
            with self.assertRaisesRegex(ValueError,'digest or byte count'):self.acquire(fetch_change=change)
            self.assertFalse((self.root/'inputs').exists())
        key='tui-proof';entry=self.plan['artifacts'][key];row=self.rows[entry['id']]
        cases=[self.zip({'../outside':b'bad'}),self.zip({'/absolute':b'bad'}),
               self.zip({'a\\b':b'bad'}),self.zip({'a':b'link'},symlink='a'),
               self.zip({'a':b'one'},duplicate='a'),self.zip({'a':b'one'},alias=True),
               self.zip({'a':b'file','a/child':b'conflict'})]
        for number,data in enumerate(cases):
            self.archives[entry['id']]=data;row['digest']='sha256:'+hashlib.sha256(data).hexdigest();row['size_in_bytes']=len(data)
            with self.subTest(case=number),self.assertRaises(ValueError):self.acquire()
            self.assertFalse((self.root/'inputs').exists());self.assertFalse((self.root/'outside').exists())

    def test_role_inventory_and_unqualified_saved_plan_are_not_transport_authority(self):
        original=copy.deepcopy(self.plan)
        for fault in ('name','run','focused','pending'):
            plan=copy.deepcopy(original)
            if fault=='name':plan['artifacts']['android-core']['name']='android-manager'
            elif fault=='run':plan['artifacts']['android-core']['run_id']=12
            elif fault=='focused':plan['runs']['tui']['display_title']='Focused Native native-api'
            else:plan['runs']['manager']['status']='in_progress'
            with self.subTest(fault=fault),self.assertRaises(ValueError):self.acquire(plan=plan)
            self.assertEqual(self.calls,[]);self.assertFalse((self.root/'inputs').exists())
        key='manager-proof';entry=self.plan['artifacts'][key]
        data=self.zip({'wrong.json':b'{}'});self.archives[entry['id']]=data
        self.rows[entry['id']].update(digest='sha256:'+hashlib.sha256(data).hexdigest(),size_in_bytes=len(data))
        with self.assertRaisesRegex(ValueError,'role inventory'):self.acquire()
        self.assertFalse((self.root/'inputs').exists())

    def test_component_run_rechecked_after_transfer_and_existing_inputs_preserved(self):
        visits=0
        def change(path,value):
            nonlocal visits
            if path=='actions/runs/12':
                visits+=1
                if visits==2:value['conclusion']='failure'
        with self.assertRaisesRegex(ValueError,'no longer qualifies'):self.acquire(change=change)
        self.assertFalse((self.root/'inputs').exists())
        inputs=self.root/'inputs';inputs.mkdir();(inputs/'owned').write_bytes(b'preserve')
        with self.assertRaises(FileExistsError):self.acquire()
        self.assertEqual((inputs/'owned').read_bytes(),b'preserve')


if __name__=='__main__':unittest.main()
