"""Immutable global sequence reservations in the existing publication repository.

Read-only qualification may list occupied sequences. Only the separately
explicitly authorized publication job creates a tag; neither operation signs.
"""
import argparse
from datetime import datetime,timezone,date
import json
from pathlib import Path
import re
from qualification_signal import api,GitHubFailure
from component_inputs import git
from component_revisions import assign,selected,candidate_digest,validate_components,MAX_REVISION
from runtime_inputs import canonical

PREFIX='codex-release-reservation-s'
LIMIT=(1<<64)-1


def validate(value):
    fields={'schema','source_sha','main_sha','generation_id','sequence','candidate_sha256','upstream_version',
            'components','composition_revision','release_date'}
    if (not isinstance(value,dict) or set(value)!=fields or value['schema']!='codex-release-reservation-v1'
            or not isinstance(value['source_sha'],str) or re.fullmatch('[0-9a-f]{40}',value['source_sha']) is None
            or not isinstance(value['main_sha'],str) or re.fullmatch('[0-9a-f]{40}',value['main_sha']) is None
            or not isinstance(value['candidate_sha256'],str) or re.fullmatch('[0-9a-f]{64}',value['candidate_sha256']) is None
            or type(value['sequence']) is not int or not 0<value['sequence']<=LIMIT
            or not isinstance(value['generation_id'],str)
            or re.fullmatch(r'(?:core-v|full-v)[a-z0-9][a-z0-9._-]{0,126}',value['generation_id']) is None
            or not value['generation_id'].endswith('-s'+str(value['sequence']))
            or not value['generation_id'].startswith(('core-v','full-v'))
            or not isinstance(value['upstream_version'],str)
            or re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+',value['upstream_version']) is None
            or type(value['composition_revision']) is not int or not 0<value['composition_revision']<=MAX_REVISION
            or not isinstance(value['release_date'],str)
            or date.fromisoformat(value['release_date']).isoformat()!=value['release_date']):
        raise ValueError('invalid release reservation')
    validate_components(value['components'])
    return value


def snapshot(request=api):
    refs=request('git/matching-refs/tags/')
    if not isinstance(refs,list) or len(refs)>10000:raise ValueError('incomplete or unbounded release tag inventory')
    occupied=set();records=[]
    for ref in refs:
        name=ref.get('ref','').removeprefix('refs/tags/')
        match=re.fullmatch(r'(?:core-v|full-v|local-(?:base-)?(?:relay-)?hosted-).*-s([1-9][0-9]*)',name)
        reserved=re.fullmatch(re.escape(PREFIX)+r'([1-9][0-9]*)',name)
        if match or reserved:
            number=int((reserved or match)[1])
            if number>LIMIT:raise ValueError('reserved release sequence overflow')
            occupied.add(number)
        if reserved:
            target=ref.get('object',{})
            if target.get('type')!='tag' or re.fullmatch('[0-9a-f]{40}',target.get('sha','')) is None:
                raise ValueError('reservation must be an annotated immutable tag')
            tag=request('git/tags/'+target['sha'])
            message=tag.get('message','')
            if len(message.encode())>16384:raise ValueError('reservation message exceeds bound')
            from release_components import unique
            value=validate(json.loads(message,object_pairs_hook=unique))
            if (tag.get('tag')!=name or tag.get('object',{}).get('type')!='commit'
                    or tag['object'].get('sha')!=value['source_sha'] or value['sequence']!=number):
                raise ValueError('reservation tag does not bind its exact source')
            records.append(value)
    return {'schema':'codex-release-reservations-v1','occupied_sequences':sorted(occupied),'reservations':records}


def signed_history(main_sha,authority,directory):
    from release_heads import authenticate,download
    from release_components import COMPONENT,component
    heads,_=authenticate(main_sha,'full',authority,directory)
    history=[]
    for lane,head in heads.items():
        folder=directory/lane
        from release_publication import canonical_release_base
        base=canonical_release_base(head['generation'])
        if not download(base+COMPONENT,folder/COMPONENT,16384,optional=True):continue
        download(base+COMPONENT+'.sig',folder/(COMPONENT+'.sig'),64)
        value=component(folder,head['generation'],authority)
        if 'components' in value:
            history.append({'upstream_version':head['version'],'components':value['components'],
                            'composition_revision':value['composition_revision']})
    return history


def reserve(source,candidate,sequence,main_sha,authority,directory,request=api,history_reader=signed_history):
    from release_control import release_authorized
    source_sha=git(source,'rev-parse','HEAD').decode().strip()
    version,current=selected(source,candidate)
    if request('git/ref/heads/main')['object']['sha']!=main_sha:
        raise ValueError('publication main moved before reservation')
    if not release_authorized(source_sha,main_sha,version):
        raise ValueError('exact-source publication is not separately authorized')
    state=snapshot(request)
    descriptor=json.loads((source/'release/upstream.json').read_text())
    if version!=descriptor['upstream']['version']:raise ValueError('candidate upstream differs')
    from release_preflight import parse_descriptor
    generation=parse_descriptor(candidate/'generation.meta')['generation_id']
    digest=candidate_digest(candidate)
    existing=next((r for r in state['reservations'] if r['sequence']==sequence),None)
    if existing:
        if (existing['source_sha']!=source_sha or existing['candidate_sha256']!=digest
                or existing['generation_id']!=generation):
            raise ValueError('release sequence already belongs to different inputs')
        return existing,False  # Original reservation/receipt, never a second signing request.
    if sequence<=max(state['occupied_sequences'],default=0):
        raise ValueError('release sequence is already occupied; re-evaluate candidate')
    history=history_reader(main_sha,authority,directory)+state['reservations']
    revisions=assign(version,current,history)
    value=validate({'schema':'codex-release-reservation-v1','source_sha':source_sha,'main_sha':main_sha,
                    'generation_id':generation,'sequence':sequence,'candidate_sha256':digest,
                    **revisions,'release_date':datetime.now(timezone.utc).date().isoformat()})
    name=PREFIX+str(sequence)
    if request('git/ref/heads/main')['object']['sha']!=main_sha:
        raise ValueError('publication main moved while preparing reservation')
    tag=request('git/tags',payload={'tag':name,'message':canonical(value).decode(),
                                   'object':source_sha,'type':'commit'})
    try:
        request('git/refs',payload={'ref':'refs/tags/'+name,'sha':tag['sha']})
    except GitHubFailure as error:
        if error.detail not in {'HTTP 409','HTTP 422'}:raise
        # No force/update path: a competing reservation must be observed.
        current=next((r for r in snapshot(request)['reservations'] if r['sequence']==sequence),None)
        if current!=value:raise ValueError('concurrent release reservation conflicts') from error
        return current,False
    observed=next((r for r in snapshot(request)['reservations'] if r['sequence']==sequence),None)
    if observed!=value:raise ValueError('release reservation readback differs')
    return value,True


def verify_candidate(value,source,candidate,sequence):
    validate(value)
    if (value['sequence']!=int(sequence) or value['source_sha']!=git(source,'rev-parse','HEAD').decode().strip()
            or value['candidate_sha256']!=candidate_digest(candidate)):
        raise ValueError('reserved signing inputs differ')
    version,current=selected(source,candidate)
    if value['upstream_version']!=version or any(
            {key:data for key,data in value['components'][name].items() if key!='revision'}!=entry
            for name,entry in current.items()) or set(current)!=set(value['components']):
        raise ValueError('reserved component inputs differ')
    return value


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('snapshot','reserve'))
    parser.add_argument('--source',type=Path);parser.add_argument('--candidate',type=Path)
    parser.add_argument('--sequence',type=int);parser.add_argument('--main-sha')
    parser.add_argument('--authority',type=Path);parser.add_argument('--history-root',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if args.action=='snapshot':value=snapshot()
    else:
        if any(getattr(args,k) is None for k in ('source','candidate','sequence','main_sha','authority','history_root')):
            parser.error('reserve requires exact source, candidate, sequence, main, authority and history root')
        value,created=reserve(args.source,args.candidate,args.sequence,args.main_sha,args.authority,args.history_root)
        print('created='+str(created).lower())
    args.output.write_bytes(canonical(value))

if __name__=='__main__':main()
