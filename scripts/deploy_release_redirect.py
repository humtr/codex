#!/usr/bin/env python3
"""Explicitly deploy/read back the credential-free fixed release redirect Worker."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import secrets
from urllib.error import HTTPError
from urllib.request import Request, urlopen

NAME='codex-releases'
SOURCE=Path(__file__).resolve().parents[1]/'release/asset-redirect.mjs'


def credentials(path):
    values={}
    for line in path.read_text().splitlines():
        line=line.strip()
        if not line or line.startswith('#') or '=' not in line:continue
        key,value=line.removeprefix('export ').split('=',1)
        values[key.strip()]=value.strip().strip('\"\'')
    account=values['CLOUDFLARE_ACCOUNT_ID'];token=values['CLOUDFLARE_API_TOKEN']
    if re.fullmatch('[0-9a-f]{32}',account) is None or not token:raise ValueError('invalid account configuration')
    return account,token


def deploy(path, previous_source=None):
    account,token=credentials(path)
    prefix='https://api.cloudflare.com/client/v4/accounts/'+account+'/workers/'
    def call(relative, method='GET', data=None, content_type=None, raw=False):
        headers={'Authorization':'Bearer '+token}
        if content_type:headers['Content-Type']=content_type
        try:
            with urlopen(Request(prefix+relative,method=method,data=data,headers=headers),timeout=60) as response:
                body=response.read(2000001)
        except HTTPError as error:
            raise RuntimeError('Cloudflare '+method+' failed: HTTP '+str(error.code)) from None
        if len(body)>2000000:raise ValueError('oversized deployment response')
        if raw:return body
        value=json.loads(body)
        if not value.get('success'):raise RuntimeError('Cloudflare rejected operation')
        return value['result']
    if call('subdomain').get('subdomain') != 'humtr':raise ValueError('unexpected Workers account origin')
    existing=next((script for script in call('scripts') if script['id']==NAME),None)
    code=SOURCE.read_bytes()
    if existing:
        old=call('scripts/'+NAME,raw=True)
        if code not in old and (previous_source is None or previous_source.read_bytes() not in old):
            raise ValueError('existing Worker differs from the explicitly supplied previous source')
    if existing is None or code not in old:
        boundary='codex-'+secrets.token_hex(16)
        metadata=json.dumps({'main_module':'asset-redirect.mjs','compatibility_date':'2026-10-01','bindings':[]}).encode()
        parts=[]
        for name,filename,kind,value in [('metadata',None,'application/json',metadata),
              ('asset-redirect.mjs','asset-redirect.mjs','application/javascript+module',code)]:
            disposition='form-data; name="'+name+'"'+('' if filename is None else '; filename="'+filename+'"')
            parts.append(('--'+boundary+'\r\nContent-Disposition: '+disposition+'\r\nContent-Type: '+kind+'\r\n\r\n').encode()+value+b'\r\n')
        body=b''.join(parts)+('--'+boundary+'--\r\n').encode()
        call('scripts/'+NAME,'PUT',body,'multipart/form-data; boundary='+boundary)
    call('scripts/'+NAME+'/subdomain','POST',json.dumps({'enabled':True,'previews_enabled':False}).encode(),'application/json')
    if code not in call('scripts/'+NAME,raw=True):raise ValueError('deployed Worker readback differs')
    return {'worker':NAME,'origin':'https://codex-releases.humtr.workers.dev',
            'source_sha256':hashlib.sha256(code).hexdigest(),'deployed_source_readback':True}


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--credentials',type=Path,required=True)
    parser.add_argument('--previous-source',type=Path,help='Exact previously deployed source; required for a bounded upgrade')
    args=parser.parse_args()
    print(json.dumps(deploy(args.credentials,args.previous_source)))
