"""Formal consumers must reuse actual accepted bytes, never relabel or rebuild."""
import copy
import hashlib
import io
import json
from pathlib import Path
import stat
import struct
import tempfile
import tarfile
import unittest
from unittest.mock import patch
import zipfile

import component_artifact as component
import qualified_component as qualified

SOURCE='a'*40
ORIGINAL='b'*40


class QualifiedComponentTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        for name,value in [('fingerprint','c'*64),('inputs',{'rust':'1.98.0'})]:
            mocked=patch.object(component,name,return_value=value);mocked.start();self.addCleanup(mocked.stop)
        source=patch.object(qualified,'git',return_value=(SOURCE+chr(10)).encode())
        source.start();self.addCleanup(source.stop)

    def fixture(self,name):
        original=self.root/('original-'+name);original.mkdir()
        data=bytearray(256);data[:6]=bytes([127,69,76,70,2,1]);struct.pack_into('<H',data,18,183)
        struct.pack_into('<Q',data,32,64);struct.pack_into('<HH',data,54,56,1)
        struct.pack_into('<I',data,64,3);struct.pack_into('<Q',data,72,128)
        linker='/system/bin/linker64'.encode()+bytes([0]);struct.pack_into('<Q',data,96,len(linker));data[128:128+len(linker)]=linker
        binary=original/component.COMPONENTS[name][1];binary.write_bytes(data);binary.chmod(0o755)
        with patch.object(component,'git',return_value=(ORIGINAL+chr(10)).encode()):
            production=component.record(self.root,name,original,'17')
        transfer=self.root/('original-transfer-'+name);component.pack(self.root,name,original,transfer)
        return production,(transfer/'component.tar').read_bytes()

    def zip(self,data,extra=None):
        stream=io.BytesIO()
        with zipfile.ZipFile(stream,'w',compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr('component.tar',data)
            if extra is not None:archive.writestr(extra,b'unselected')
        return stream.getvalue()

    def authority(self,name,archive):
        run={'id':31,'path':'.github/workflows/'+qualified.WORKFLOWS[name],'head_sha':SOURCE,
             'head_branch':'rewrite/rust-core','event':'workflow_dispatch','status':'completed',
             'conclusion':'success','repository':{'full_name':'humtr/codex'}}
        row={'id':41,'name':'android-'+name,'expired':False,'size_in_bytes':len(archive),
             'digest':'sha256:'+hashlib.sha256(archive).hexdigest(),'workflow_run':{'id':31,'head_sha':SOURCE}}
        return run,row

    def consume(self,name,archive,run,row,*,mutation=None):
        calls=[]
        def lookup(workflow,source):
            self.assertEqual(workflow,qualified.WORKFLOWS[name]);self.assertEqual(source,SOURCE)
            return run,[run]
        def request(path):
            calls.append(path)
            if path.endswith('/artifacts?per_page=100'):
                result={'total_count':1,'artifacts':[copy.deepcopy(row)]}
            elif path=='actions/artifacts/41':result=copy.deepcopy(row)
            elif path=='actions/runs/31':result=copy.deepcopy(run)
            else:self.fail('unexpected authority access '+path)
            if mutation:mutation(path,result)
            return result
        def fetch(selected,path):
            self.assertEqual(selected,row);path.write_bytes(archive)
        result=qualified.acquire(self.root,name,self.root/'received',self.root/'transfer',request,lookup,fetch)
        return result,calls

    def test_formal_main_consumer_keeps_original_source_run_and_exact_qualified_bytes(self):
        for name in ('core','manager'):
            with self.subTest(component=name):
                production,tar=self.fixture(name);archive=self.zip(tar);run,row=self.authority(name,archive)
                result,calls=self.consume(name,archive,run,row)
                self.assertEqual(result['consumer_source'],SOURCE)
                self.assertEqual(result['qualification_run_id'],31)
                self.assertEqual(result['qualification_artifact_id'],41)
                self.assertEqual(result['qualification_archive_digest'],row['digest'])
                self.assertEqual(result['qualification_archive_bytes'],len(archive))
                self.assertEqual(result['production'],production)
                self.assertEqual(result['production']['producer_source'],ORIGINAL)
                self.assertEqual(result['production']['production_run_id'],'17')
                self.assertEqual(component.verify(self.root,name,self.root/'received'),production)
                self.assertEqual(calls[-1],'actions/runs/31')
                import shutil
                shutil.rmtree(self.root/'received');shutil.rmtree(self.root/'transfer')

    def test_missing_foreign_pending_failed_or_wrong_source_qualification_cannot_compile(self):
        _,tar=self.fixture('core');archive=self.zip(tar);run,row=self.authority('core',archive)
        for change in [None,{'head_sha':ORIGINAL},{'head_branch':'main'},{'event':'pull_request'},
                       {'status':'in_progress'},{'conclusion':'failure'},{'repository':{'full_name':'foreign/repo'}},
                       {'path':'.github/workflows/source-acceptance.yml'}]:
            altered=None if change is None else {**run,**change}
            with self.subTest(change=change),self.assertRaisesRegex(ValueError,'no successful exact-source'):
                self.consume('core',archive,altered,row)
            self.assertFalse((self.root/'transfer').exists());self.assertFalse((self.root/'received').exists())

    def test_artifact_listing_digest_authority_and_byte_identity_are_mandatory(self):
        _,tar=self.fixture('core');archive=self.zip(tar);run,row=self.authority('core',archive)
        mutations=[
            lambda path,value: value.update(total_count=2) if path.endswith('per_page=100') else None,
            lambda path,value: value.update(artifacts=[row,row],total_count=2) if path.endswith('per_page=100') else None,
            lambda path,value: value['artifacts'][0].update(expired=True) if path.endswith('per_page=100') else None,
            lambda path,value: value.update(digest='sha256:'+'d'*64) if path=='actions/artifacts/41' else None,
            lambda path,value: value.update(workflow_run={'id':32,'head_sha':SOURCE}) if path=='actions/artifacts/41' else None,
            lambda path,value: value.update(conclusion='failure') if path=='actions/runs/31' else None]
        for number,mutation in enumerate(mutations):
            with self.subTest(fault=number),self.assertRaises(ValueError):
                self.consume('core',archive,run,row,mutation=mutation)
            self.assertFalse((self.root/'received').exists());self.assertFalse((self.root/'transfer').exists())
        for wrong in [{**row,'digest':'sha256:'+'e'*64},{**row,'size_in_bytes':len(archive)+1}]:
            with self.subTest(row=wrong),self.assertRaisesRegex(ValueError,'digest or byte count'):
                self.consume('core',archive,run,wrong)
            self.assertFalse((self.root/'received').exists());self.assertFalse((self.root/'transfer').exists())

    def test_zip_inventory_and_actual_component_receipt_remain_fail_closed(self):
        _,tar=self.fixture('manager')
        alias=io.BytesIO()
        with zipfile.ZipFile(alias,'w') as archive:
            entry=zipfile.ZipInfo('component.tar');entry.create_system=3
            entry.external_attr=(stat.S_IFLNK|0o777)<<16
            archive.writestr(entry,tar)
        for fault,archive in [('extended ZIP',self.zip(tar,'../unselected')),('symlink ZIP',alias.getvalue()),
                              ('wrong component',self.zip(tar.replace(b'codex-manager',b'codex-forged!')))]:
            run,row=self.authority('manager',archive)
            with self.subTest(fault=fault),self.assertRaises((ValueError,tarfile.TarError)):
                self.consume('manager',archive,run,row)
            self.assertFalse((self.root/'received').exists());self.assertFalse((self.root/'transfer').exists())
        original=self.root/'original-manager';metadata=original/'component.json'
        value=json.loads(metadata.read_text());value['inputs_sha256']='e'*64
        from runtime_inputs import canonical
        metadata.write_bytes(canonical(value))
        stream=io.BytesIO()
        with tarfile.open(fileobj=stream,mode='w',format=tarfile.USTAR_FORMAT) as archive:
            for name in ['codex-manager','component.json']:archive.add(original/name,arcname=name)
        archive=self.zip(stream.getvalue());run,row=self.authority('manager',archive)
        with self.assertRaisesRegex(ValueError,'receipt inputs'):
            self.consume('manager',archive,run,row)
        self.assertFalse((self.root/'received').exists());self.assertFalse((self.root/'transfer').exists())

    def test_existing_destination_is_never_changed(self):
        _,tar=self.fixture('core');archive=self.zip(tar);run,row=self.authority('core',archive)
        target=self.root/'received';target.mkdir();(target/'owned').write_text('retain')
        with self.assertRaisesRegex(ValueError,'destination exists'):self.consume('core',archive,run,row)
        self.assertEqual((target/'owned').read_text(),'retain')
        self.assertFalse((self.root/'transfer').exists())


if __name__=='__main__':unittest.main()
