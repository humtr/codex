"""User attention for terminal automation failures. Never repairs or retries work."""
import argparse
import base64
import hashlib
import json
import re

from release_control import api

WORKFLOWS={'rald7-full-acceptance.yml':'rewrite/rust-core',
           'core-runtime-qualification.yml':'rewrite/rust-core',
           'manager-tui-native.yml':'rewrite/rust-core',
           'auto-release-termux.yml':'main', 'accepted-release-inputs.yml':'main'}
LABEL='release-attention'


def classify(run,jobs):
    workflow=run.get('path','').removeprefix('.github/workflows/')
    if (workflow not in WORKFLOWS or run.get('head_branch')!=WORKFLOWS[workflow]
        or run.get('repository',{}).get('full_name')!='humtr/codex'
        or run.get('status')!='completed' or re.fullmatch('[0-9a-f]{40}',run.get('head_sha','')) is None
        or run.get('event') not in ['push','schedule','workflow_dispatch']):return None
    if run.get('conclusion') not in ['failure','timed_out','startup_failure','action_required']:return None
    failed=sorted(job['name'] for job in jobs if job.get('conclusion') in ['failure','timed_out','startup_failure','action_required'])
    fingerprint=hashlib.sha256(json.dumps([workflow,run['head_sha'],failed,run['conclusion']]).encode()).hexdigest()
    return workflow,failed,fingerprint


def report(run_id):
    if not str(run_id).isdigit():raise ValueError('invalid completion run')
    run=api('actions/runs/'+str(run_id))
    jobs=api('actions/runs/'+str(run_id)+'/jobs?per_page=100')['jobs']
    classified=classify(run,jobs)
    if classified is None:return {'state':'no-actionable-failure'}
    workflow,failed,fingerprint=classified
    marker='<!-- codex-release-attention:'+workflow+' -->'
    signature='<!-- failure:'+fingerprint+' -->'
    heads={}
    for lane,path in [('base','update-index-base-v1'),('full','update-index-v1')]:
        try:
            content=api('contents/'+path+'?ref=main')['content']
            text=base64.b64decode(content).decode()
            value=next(line.split('\t',1)[1] for line in text.splitlines() if line.startswith('generation_id\t'))
            heads[lane]=value if re.fullmatch('local-[a-z0-9._-]+',value) else 'unavailable'
        except (ValueError,KeyError,StopIteration):heads[lane]='unavailable'
    url='https://github.com/humtr/codex/actions/runs/'+str(run_id)
    body='\n'.join([marker,signature,'Automation stopped; the prior qualified installation remains unchanged.',
         '', 'Failed stage(s): '+(', '.join(failed) or run['conclusion']), 'Workflow commit: `'+run['head_sha']+'`',
         'Serving base / latest add-on repair target: `'+heads['base']+'`',
         'Serving full fallback: `'+heads['full']+'`','',
         'Action needed: inspect the failed job log, correct the failed transformation/add-on or infrastructure, '
         'and rerun qualification against the latest successful base. Do not roll base back to satisfy full.',
         '[Workflow logs]('+url+')','No completed tools, user requests or release jobs are automatically replayed.'])
    issues=api('issues?state=open&per_page=100&labels='+LABEL)
    issue=next((issue for issue in issues if marker in (issue.get('body') or '') and 'pull_request' not in issue),None)
    if issue is not None:
        if signature in issue.get('body',''):return {'state':'already-notified','issue':issue['number']}
        api('issues/'+str(issue['number']),method='PATCH',payload={'body':body})
        api('issues/'+str(issue['number'])+'/comments',payload={'body':'New automation failure requires review: [logs]('+url+'). Latest base repair target: `'+heads['base']+'`.'})
        return {'state':'attention-updated','issue':issue['number']}
    # Create a label only when absent; existing colour/description remain untouched.
    labels=api('labels?per_page=100')
    if not any(label['name']==LABEL for label in labels):
        api('labels',payload={'name':LABEL,'color':'d73a4a','description':'Automated release or qualification needs owner attention'})
    issue=api('issues',payload={'title':'[Release attention] '+workflow,'body':body,'labels':[LABEL],'assignees':['humtr']})
    return {'state':'attention-created','issue':issue['number']}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    group=parser.add_mutually_exclusive_group(required=True);group.add_argument('--run-id');group.add_argument('--reconcile',action='store_true')
    args=parser.parse_args()
    if args.reconcile:
        result=[]
        for workflow,branch in WORKFLOWS.items():
            runs=api('actions/workflows/'+workflow+'/runs?branch='+branch+'&per_page=10')['workflow_runs']
            latest=next((run for run in runs if run['status']=='completed' and run.get('event') in ['push','schedule','workflow_dispatch']),None)
            if latest is not None:result.append(report(latest['id']))
        print(json.dumps(result))
    else:print(json.dumps(report(args.run_id)))
