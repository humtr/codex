import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import release_reservations as reservations
from release_heads import plan
from component_revisions import assign

class ReservationTests(unittest.TestCase):
    def value(self,sequence=166):
        current={name:{'inputs_sha256':'b'*64,'artifact_sha256':'c'*64,'interface':'qualified-api-v1'}
                 for name in ['core','runtime']}
        return {'schema':'codex-release-reservation-v1','source_sha':'a'*40,'main_sha':'d'*40,
                'generation_id':f'core-v0.167.1-aaaaaaa-s{sequence}','sequence':sequence,
                'candidate_sha256':'e'*64,'release_date':'2026-11-12',**assign('0.167.1',current,[])}

    def test_global_sequence_includes_unpublished_and_reserved_tags(self):
        value=self.value(170)
        def request(path):
            if path=='git/matching-refs/tags/':
                return [{'ref':'refs/tags/full-v0.167.1-aaaaaaa-s165','object':{'type':'commit','sha':'a'*40}},
                        {'ref':'refs/tags/core-v0.167.1-aaaaaaa-s168','object':{'type':'commit','sha':'a'*40}},
                        {'ref':'refs/tags/codex-release-reservation-s170','object':{'type':'tag','sha':'b'*40}}]
            self.assertEqual(path,'git/tags/'+'b'*40)
            return {'tag':'codex-release-reservation-s170','object':{'type':'commit','sha':'a'*40},'message':json.dumps(value)}
        state=reservations.snapshot(request)
        self.assertEqual(state['occupied_sequences'],[165,168,170])
        heads={'full':{'generation':'full-v0.167.1-aaaaaaa-s165','version':'0.167.1','archive':'c'*64,'sequence':165}}
        self.assertEqual(plan(heads,'base',state['occupied_sequences'])['release_sequence'],'171')
        self.assertEqual(state['reservations'],[value])
        for bad in [True,0,-1,reservations.LIMIT+1]:
            with self.assertRaises(ValueError):plan(heads,'base',[bad])

    def test_reservation_is_atomic_idempotent_and_never_overwrites_another_owner(self):
        value=self.value();created=[];tags={};refs=[]
        def request(path,**kwargs):
            if path=='git/ref/heads/main':return {'object':{'sha':'d'*40}}
            if path=='git/matching-refs/tags/':return refs
            if path=='git/tags':
                self.assertEqual(kwargs['payload']['object'],'a'*40)
                tags['f'*40]=kwargs['payload'];return {'sha':'f'*40}
            if path=='git/refs':
                created.append(kwargs['payload']);refs.append({'ref':kwargs['payload']['ref'],'object':{'type':'tag','sha':'f'*40}})
                return {'ref':kwargs['payload']['ref']}
            if path=='git/tags/'+'f'*40:
                tag=tags['f'*40]
                return {'tag':tag['tag'],'object':{'type':'commit','sha':tag['object']},'message':tag['message']}
            self.fail('unexpected or non-atomic API '+path)
        current={k:{a:b for a,b in v.items() if a!='revision'} for k,v in value['components'].items()}
        from tempfile import TemporaryDirectory
        with TemporaryDirectory() as temp:
            source=Path(temp);(source/'release').mkdir();(source/'release/upstream.json').write_text(json.dumps({'upstream':{'version':'0.167.1'}}))
            with patch.object(reservations,'git',return_value=('a'*40+'\n').encode()), \
                 patch.object(reservations,'selected',return_value=('0.167.1',current)), \
                 patch.object(reservations,'candidate_digest',return_value='e'*64), \
                 patch('release_control.release_authorized',return_value=True), \
                 patch('release_preflight.parse_descriptor',return_value={'generation_id':value['generation_id']}):
                first,new=reservations.reserve(source,source,166,'d'*40,source,source,request,lambda *args:[])
                self.assertTrue(new);self.assertEqual(len(created),1)
                second,new=reservations.reserve(source,source,166,'d'*40,source,source,request,lambda *args:[])
                self.assertFalse(new);self.assertEqual(second,first);self.assertEqual(len(created),1)
                with patch.object(reservations,'candidate_digest',return_value='f'*64):
                    with self.assertRaisesRegex(ValueError,'different inputs'):
                        reservations.reserve(source,source,166,'d'*40,source,source,request,lambda *args:[])
                self.assertEqual(len(created),1)
                with patch('release_control.release_authorized',return_value=False):
                    with self.assertRaisesRegex(ValueError,'not separately authorized'):
                        reservations.reserve(source,source,167,'d'*40,source,source,request,lambda *args:[])
                self.assertEqual(len(created),1)

    def test_reservation_record_rejects_owner_metadata_and_revision_tampering(self):
        value=self.value();self.assertEqual(reservations.validate(value),value)
        for change in [{'sequence':165},{'main_sha':'main'},{'candidate_sha256':'x'*64},
                       {'components':{}},{'composition_revision':False}]:
            with self.subTest(change=change),self.assertRaises(ValueError):reservations.validate({**value,**change})

if __name__=='__main__':unittest.main()
