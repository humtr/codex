"""Full proof must precede public inputs; signing still binds actual bytes."""
import base64
import copy
from contextlib import contextmanager, ExitStack
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import full_qualification as full
import release_control as control
from runtime_inputs import canonical

ROOT=Path(__file__).resolve().parents[2]
SHA='a'*40
MAIN='b'*40
OLD='c'*40
VERSION=control.load(ROOT/'release/upstream.json')['upstream']['version']


def run_record(workflow,number=99):
    return {'id':number,'path':'.github/workflows/'+workflow,'head_sha':SHA,
            'head_branch':control.SOURCE_BRANCH,'event':'workflow_dispatch',
            'status':'completed','conclusion':'success','display_title':'Full native qualification',
            'repository':{'full_name':control.REPO}}


def receipt():
    inventory={name:{'sha256':hashlib.sha256(name.encode()).hexdigest(),'mode':mode,'bytes':len(name)}
               for name,mode in full.EXTENDED_FILES.items()}
    components={name:{'inputs_sha256':'d'*64,'artifact_sha256':inventory[path]['sha256'],
                      'interface':'test-interface-v1'}
                for name,path in {'core':'core','runtime':'runtime','manager':'manager','tui':'helpers/2'}.items()}
    candidate={name:{'sha256':entry['sha256'],'mode':int(entry['mode'],8),'size':entry['bytes']}
               for name,entry in inventory.items()}
    outputs={name:hashlib.sha256(output).hexdigest() for name,output in {
        'core':b'usage: codex update [--wait | --no-wait]\n',
        'manager':b'codex-manager-artifact-v1\ncore_api=codex-manager-core-v1\n',
        'runtime':('codex-cli '+VERSION+'\n').encode(),'tui':('codex-cli '+VERSION+'\n').encode()}.items()}
    return {'schema':'codex-full-qualification-v1','source_sha':SHA,
            'upstream_version':VERSION,'generation_id':'full-qualification-'+VERSION.replace('.','-')+'-'+SHA[:12],
            'run_id':'99','component_runs':{'core':11,'manager':12,'tui':13},'components':components,
            'inventory':inventory,'candidate_sha256':hashlib.sha256(canonical(candidate)).hexdigest(),
            'descriptor_sha256':'e'*64,'composition_sha256':'f'*64,
            'checks':['qualified-component-inputs','full-android-interface-probes','runtime-and-tui-version'],
            'execution_outputs':outputs}


class FullReceiptTests(unittest.TestCase):
    def read(self,directory,*,change=None,missing=False):
        value=receipt();run=run_record('full-qualification.yml')
        page={'total_count':1,'artifacts':[{'id':101,'name':'full-composition-qualification','expired':False}]}
        components={str(number):run_record(full.WORKFLOWS[name],number)
                    for name,number in value['component_runs'].items()}
        current=copy.deepcopy(run);calls=[]
        if change:change(value,run,page,components,current)
        def lookup(workflow,sha):
            self.assertEqual((workflow,sha),('full-qualification.yml',SHA))
            return (None,[]) if missing else (run,[run])
        def request(path):
            calls.append(('read',path))
            if path=='actions/runs/99/artifacts?per_page=100':return page
            if path=='actions/runs/99':return current
            if path.startswith('actions/runs/'):return components[path.rsplit('/',1)[1]]
            self.fail('unexpected read '+path)
        def fetch(*args):
            calls.append(('download',args))
            self.assertEqual(args[:6],('run','download','99','--repo',control.REPO,'--name'))
            (Path(args[-1])/'full-qualification.json').write_bytes(canonical(value))
        result=full.read_qualified_full(SHA,directory,request=request,lookup=lookup,fetch=fetch)
        return result,calls

    def test_full_receipt_and_each_component_are_revalidated_without_publication(self):
        with tempfile.TemporaryDirectory() as temporary:
            result,calls=self.read(Path(temporary)/'proof')
            self.assertEqual(result[1],receipt())
            self.assertEqual([path for kind,path in calls if kind=='read'],
                ['actions/runs/99/artifacts?per_page=100','actions/runs/99','actions/runs/11',
                 'actions/runs/12','actions/runs/13'])
            missing,calls=self.read(Path(temporary)/'missing',missing=True)
            self.assertIsNone(missing);self.assertEqual(calls,[])
            self.assertFalse((Path(temporary)/'missing').exists())

    def test_failed_pending_foreign_or_focused_component_cannot_supply_full_authority(self):
        mutations=[
            lambda value,run,page,components,current:current.update(status='in_progress'),
            lambda value,run,page,components,current:current.update(id=100),
            lambda value,run,page,components,current:components['11'].update(id=100),
            lambda value,run,page,components,current:components['11'].update(conclusion='failure'),
            lambda value,run,page,components,current:components['12'].update(head_sha=OLD),
            lambda value,run,page,components,current:components['12'].update(head_branch='main'),
            lambda value,run,page,components,current:components['13'].update(display_title='Focused Native native-api'),
            lambda value,run,page,components,current:components['13'].update(repository={'full_name':'foreign/repo'}),
        ]
        for index,change in enumerate(mutations):
            with self.subTest(index=index),tempfile.TemporaryDirectory() as temporary,self.assertRaises(ValueError):
                self.read(Path(temporary)/'proof',change=change)

    def test_missing_ambiguous_incomplete_or_expired_artifact_and_malformed_receipts_fail_closed(self):
        mutations=[
            lambda value,run,page,components,current:page['artifacts'][0].update(expired=True),
            lambda value,run,page,components,current:page.update(artifacts=[],total_count=0),
            lambda value,run,page,components,current:page.update(artifacts=page['artifacts']*2,total_count=2),
            lambda value,run,page,components,current:page.update(total_count=2),
            lambda value,run,page,components,current:value.update(source_sha=OLD),
            lambda value,run,page,components,current:value.update(candidate_sha256='0'*64),
            lambda value,run,page,components,current:value['components']['manager'].update(artifact_sha256='0'*64),
            lambda value,run,page,components,current:value['inventory']['runtime'].update(mode='0777'),
            lambda value,run,page,components,current:value['inventory']['core'].update(bytes=True),
            lambda value,run,page,components,current:value['component_runs'].update(core=True),
            lambda value,run,page,components,current:value['execution_outputs'].pop('manager'),
            lambda value,run,page,components,current:value['execution_outputs'].update(runtime='0'*64),
            lambda value,run,page,components,current:value.update(checks=['compiled']),
        ]
        for index,change in enumerate(mutations):
            with self.subTest(index=index),tempfile.TemporaryDirectory() as temporary,self.assertRaises(ValueError):
                self.read(Path(temporary)/'proof',change=change)


class FullPublicationTests(unittest.TestCase):
    @staticmethod
    def frontend(number=13):
        return {'source':SHA,'run_id':str(number),'sha256':'d'*64,
                'input_url':'https://github.com/humtr/codex/releases/download/owned/input.tar.gz',
                'input_sha256':'e'*64}

    @contextmanager
    def controller(self,*,lane='full',proof=True,previous=OLD,registered=None,frontend=True,moved=False):
        effects=[];selection_calls=[];refs=0
        selected=self.frontend() if frontend else None
        def api(path,**kwargs):
            nonlocal refs
            if path=='git/ref/heads/'+control.SOURCE_BRANCH:return {'object':{'sha':SHA}}
            if path=='git/ref/heads/main':
                refs+=1
                return {'object':{'sha':OLD if moved and refs>1 else MAIN}}
            if path.startswith('contents/release-source-v1'):
                return {'content':base64.b64encode((previous+'\n').encode()).decode()}
            if path.startswith('actions/workflows/auto-release-termux.yml/runs'):return {'workflow_runs':[]}
            self.fail('unexpected API, possibly a public effect: '+path)
        def ensure(source,sha,root,pinned,*,pending,required_run_id=None):
            self.assertEqual((source,sha),(ROOT,SHA))
            selection_calls.append(required_run_id)
            choice=self.frontend(required_run_id) if required_run_id is not None else selected
            if choice is not None:pending.append({'run_id':choice['run_id'],'root':root})
            return {**control.load(ROOT/'release/upstream.json'),'frontend':choice}
        def publish(item):
            self.assertTrue(item['root'].exists())
            effects.append(('publish',item['run_id']))
        with ExitStack() as stack:
            def mock(name,**kwargs):return stack.enter_context(patch.object(control,name,**kwargs))
            stack.enter_context(patch.object(control.subprocess,'check_output',return_value=SHA+'\n'))
            mock('api',side_effect=api);mock('release_authorized',return_value=True)
            mock('registered_frontend',return_value=registered)
            mock('same_product_inputs',return_value=False)
            mock('successful_run',return_value=({'id':1},[]))
            mock('ensure_frontend',side_effect=ensure);mock('caller_compatible',return_value=True)
            mock('release_lane',return_value=lane)
            gate=mock('read_qualified_full',return_value=(run_record('full-qualification.yml'),receipt()) if proof else None)
            publisher=mock('publish_frontend_input',side_effect=publish)
            commit=mock('commit_files',side_effect=lambda *args:effects.append(('commit',args)) or 'f'*40)
            dispatch=mock('dispatch',side_effect=lambda *args:effects.append(('dispatch',args)))
            yield {'effects':effects,'gate':gate,'publish':publisher,'commit':commit,
                   'dispatch':dispatch,'selections':selection_calls}

    def test_full_only_waits_without_public_assets_refs_or_release_dispatch(self):
        for previous,registered,request in [(OLD,None,None),(SHA,self.frontend(),VERSION)]:
            with self.subTest(previous=previous),self.controller(proof=False,previous=previous,registered=registered) as state:
                result=control.run(ROOT,request)
                self.assertEqual(result,{'state':'awaiting-exact-proof','workflow':'full-qualification.yml','source':SHA})
                self.assertEqual(state['effects'],[])
                state['gate'].assert_called_once()

    def test_full_only_never_silently_falls_back_to_core_when_frontend_missing(self):
        with self.controller(frontend=False) as state:
            result=control.run(ROOT)
            self.assertEqual(result,{'state':'awaiting-exact-proof','workflow':'manager-tui-native.yml','source':SHA})
            self.assertEqual(state['effects'],[])

    def test_both_may_publish_only_core_without_optional_publication_or_full_authority(self):
        with self.controller(lane='both',proof=False) as state:
            result=control.run(ROOT)
            self.assertEqual(result['state'],'accepted-product-pinned-public-proof-started')
            state['publish'].assert_not_called()
            files=state['commit'].call_args.args[2]
            self.assertEqual(files['release-source-v1'],(SHA+'\n').encode())
            self.assertEqual(json.loads(files['release-frontend-v1']),{'source':SHA,'frontend':None})
            self.assertEqual(state['dispatch'].call_args.args[2]['variant'],'base')

    def test_core_does_not_query_full_and_complete_full_publishes_only_after_admission(self):
        with self.controller(lane='both',frontend=False) as state:
            control.run(ROOT)
            state['gate'].assert_not_called();state['publish'].assert_not_called()
            self.assertEqual(state['dispatch'].call_args.args[2]['variant'],'base')
        with self.controller() as state:
            control.run(ROOT)
            self.assertEqual([effect[0] for effect in state['effects']],['publish','commit','dispatch'])
            self.assertEqual(state['dispatch'].call_args.args[2]['variant'],'full')
            state['gate'].assert_called_once()

    def test_full_uses_exact_tui_selected_by_proof_and_rejects_unavailable_bytes(self):
        with self.controller() as state:
            proof=receipt();proof['component_runs']['tui']=456
            state['gate'].return_value=(run_record('full-qualification.yml'),proof)
            control.run(ROOT)
            self.assertEqual(state['selections'],[None,456])
            self.assertEqual(state['effects'][0],('publish','456'))
            files=state['commit'].call_args.args[2]
            self.assertEqual(json.loads(files['release-frontend-v1'])['frontend']['run_id'],'456')
        with self.controller() as state:
            proof=receipt();proof['component_runs']['tui']=456
            state['gate'].return_value=(run_record('full-qualification.yml'),proof)
            with patch.object(control,'ensure_frontend',return_value={'frontend':self.frontend()}):
                with self.assertRaisesRegex(ValueError,'unavailable'):
                    control.run(ROOT)
            self.assertEqual(state['effects'],[])

    def test_full_wrong_upstream_refuses_before_any_deferred_publication(self):
        with self.controller() as state:
            proof=receipt();proof['upstream_version']='0.999.0'
            state['gate'].return_value=(run_record('full-qualification.yml'),proof)
            with self.assertRaisesRegex(ValueError,'upstream version differs'):
                control.run(ROOT)
            self.assertEqual(state['effects'],[])

    def test_moved_main_refuses_before_any_deferred_publication(self):
        with self.controller(moved=True) as state,self.assertRaisesRegex(ValueError,'main moved'):
            control.run(ROOT)
        self.assertEqual(state['effects'],[])

    def test_frontend_collection_defers_prerelease_and_immutable_readback(self):
        run=run_record('manager-tui-native.yml',13);calls=[]
        def api(path,**kwargs):
            if path.startswith('actions/workflows/manager-tui-native.yml/runs'):return {'workflow_runs':[run]}
            self.fail('unexpected API before Full admission: '+path)
        def download(*args,**kwargs):
            calls.append(args)
            self.assertEqual(args[:3],('run','download','13'))
            directory=Path(args[-1])
            if args[6]=='manager-tui-native':(directory/'codex').write_bytes(b'owned raw TUI fixture')
            else:(directory/'result').write_text('owned execution proof')
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);pending=[]
            with patch.object(control,'api',side_effect=api),patch.object(control,'gh',side_effect=download),\
                 patch.object(control,'frontend_recipe_matches',return_value=True),\
                 patch.object(control,'verify_frontend'),patch.object(control,'dispatch') as dispatch:
                selected=control.ensure_frontend(ROOT,SHA,root,None,pending=pending)
            self.assertEqual(selected['frontend']['run_id'],'13');self.assertEqual(len(pending),1)
            self.assertEqual(len(calls),2);dispatch.assert_not_called()
            self.assertFalse((pending[0]['directory']/'readback').exists())
            def publish(*args,**kwargs):
                calls.append(args)
                if args[:2]==('release','download'):
                    (Path(args[-1])/pending[0]['name']).write_bytes(pending[0]['bundle'].read_bytes())
            with patch.object(control,'api',return_value=[]),patch.object(control,'gh',side_effect=publish):
                control.publish_frontend_input(pending[0])
            self.assertEqual([entry[:2] for entry in calls],[('run','download'),('run','download'),
                                                          ('release','create'),('release','download')])

if __name__=='__main__':unittest.main()
