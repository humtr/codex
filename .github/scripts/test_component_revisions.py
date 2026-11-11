import copy
import unittest
from component_revisions import assign,validate_components,MAX_REVISION

class RevisionTests(unittest.TestCase):
    def components(self):
        return {name:{'inputs_sha256':str(n)*64,'artifact_sha256':str(n+1)*64,'interface':'qualified-api-v1'}
                for n,name in enumerate(('core','runtime','manager','tui'),1)}

    def test_revision_is_per_component_and_never_release_sequence(self):
        current=self.components();first=assign('0.167.1',current,[])
        self.assertEqual({e['revision'] for e in first['components'].values()},{1})
        self.assertEqual(assign('0.167.1',current,[first]),first)
        changed=copy.deepcopy(current);changed['manager']['inputs_sha256']='f'*64
        second=assign('0.167.1',changed,[first])
        self.assertEqual(second['components']['manager']['revision'],2)
        for name in ['core','runtime','tui']:self.assertEqual(second['components'][name],first['components'][name])
        self.assertEqual(second['composition_revision'],2)
        self.assertEqual(assign('0.167.2',changed,[first,second])['components']['manager']['revision'],1)
        self.assertEqual(assign('0.167.1',current,[first,second]),first)

    def test_core_and_full_share_modules_but_have_distinct_compositions(self):
        full=self.components();core={k:v for k,v in full.items() if k in {'core','runtime'}}
        base=assign('0.167.1',core,[]);complete=assign('0.167.1',full,[base])
        for name in core:self.assertEqual(complete['components'][name],base['components'][name])
        self.assertEqual(complete['composition_revision'],1)
        self.assertEqual(assign('0.167.1',core,[base,complete]),base)

    def test_unpublished_reservations_consume_revisions_and_conflicts_refuse(self):
        current=self.components();first=assign('0.167.1',current,[])
        changed=copy.deepcopy(current);changed['tui']['artifact_sha256']='f'*64
        reserved=assign('0.167.1',changed,[first])
        next_input=copy.deepcopy(changed);next_input['tui']['inputs_sha256']='e'*64
        self.assertEqual(assign('0.167.1',next_input,[first,reserved])['components']['tui']['revision'],3)
        corrupt=copy.deepcopy(first);corrupt['components']['core']['artifact_sha256']='a'*64
        with self.assertRaisesRegex(ValueError,'conflicting'):assign('0.167.1',current,[first,corrupt])
        for invalid in [0,-1,True,MAX_REVISION+1]:
            corrupt=copy.deepcopy(first);corrupt['components']['core']['revision']=invalid
            with self.assertRaises(ValueError):validate_components(corrupt['components'])

if __name__=='__main__':unittest.main()
