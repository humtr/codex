"""Opt-in upstream profile and command-backed local relay authentication."""
import argparse
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import tomllib
from urllib.parse import urlparse
from open_dual_trial import record, owner
from linux_desktop_trial import private, write

BEGIN='# codex-web-experiment ChatGPT profile begin'
END='# codex-web-experiment ChatGPT profile end'
PROFILE='chatgpt'
PROVIDER='chatgpt_desktop_trial'


def server_record(path):
    value=record(path);owner({**value,'start':int(value['start'])})
    u=urlparse(value['baseUrl'])
    assert u.scheme=='http' and u.hostname=='127.0.0.1' and u.port and u.path=='/v1' and not u.username and not u.password and not u.query and not u.fragment
    assert value['model']=='chatgpt-desktop/instant' and re.fullmatch('[0-9a-f]{64}',value['token'])
    return value


def install(server,codex_home,template,runtime=None):
    value=server_record(server)
    home=private(codex_home)
    config=home/'config.toml'
    if config.exists() or config.is_symlink():
        state=config.lstat();assert config.is_file() and not config.is_symlink() and state.st_uid==os.getuid() and not state.st_mode&0o077
        previous=config.read_text()
    else:previous=''
    if BEGIN in previous:
        assert previous.count(BEGIN)==1 and previous.count(END)==1
        a=previous.index(BEGIN);b=previous.index(END,a)+len(END);previous=previous[:a]+previous[b:]
    parsed=tomllib.loads(previous)
    target=home/(PROFILE+'.config.toml')
    if target.exists() or target.is_symlink():
        state=target.lstat();assert target.is_file() and not target.is_symlink() and state.st_uid==os.getuid() and not state.st_mode&0o077
        assert target.read_text().startswith(BEGIN),'existing_user_profile_conflict'

    data=private(Path.home()/'.local/share/codex-web-experiment/native-provider')
    binding=data/'server.json';write(binding,value)
    model=json.loads(template.read_text())['models'][0]
    model.update({'slug':value['model'],'display_name':'ChatGPT · Instant','description':'Actual ChatGPT Chat mode through the local Linux desktop bridge (experimental)',
                  'context_window':32768,'max_context_window':32768,'auto_compact_token_limit':None,
                  'prefer_websockets':False,'supports_reasoning_effort_updates':False,'default_reasoning_level':'none',
                  'supported_reasoning_levels':[{'effort':'none','description':'ChatGPT Instant'}],
                  'tool_mode':'direct','multi_agent_version':'v1','model_messages':{'instructions_template':'You are Codex, a coding agent. Follow the user task and execute only advertised tools through the native client.','instructions_variables':None},'input_modalities':['text'],'supports_search_tool':True,'visibility':'list','supported_in_api':True})
    catalog=data/'models.json';write(catalog,{'models':[model]})
    helper=Path(__file__).resolve()
    auth_args=['-B',str(helper),'token',str(binding)]
    if runtime is not None:
        from desktop_lifecycle import record,endpoint
        installed=record(runtime);endpoint(installed)
        assert all(installed[k]==value[k] for k in ['baseUrl','token','model'])
        auth_args=['-B',str(Path(installed['source'])/'desktop_lifecycle.py'),'token','--runtime',str(runtime)]
    block=f'''{BEGIN}
model = {json.dumps(value['model'])}
model_provider = "{PROVIDER}"
model_reasoning_effort = "none"
model_catalog_json = {json.dumps(str(catalog))}
[model_providers.{PROVIDER}]
name = "ChatGPT desktop"
base_url = {json.dumps(value['baseUrl'])}
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
[model_providers.{PROVIDER}.auth]
command = {json.dumps(os.environ['PREFIX']+'/bin/python')}
args = {json.dumps(auth_args)}
timeout_ms = 120000
cwd = {json.dumps(str(data))}
refresh_interval_ms = 30000
{END}
'''
    updated=block;tomllib.loads(updated)
    # Remove only this experiment's earlier inline-profile block; preserve other edits.
    if config.exists() and BEGIN in config.read_text():
        baseline=previous.rstrip()+'\n'
        fd,temporary=tempfile.mkstemp(dir=home,prefix='.base-restore-')
        with os.fdopen(fd,'w') as f:f.write(baseline);f.flush();os.fsync(f.fileno())
        os.replace(temporary,config)

    fd,tmp=tempfile.mkstemp(dir=home,prefix='.chatgpt-config-')
    try:
        with os.fdopen(fd,'w') as f:f.write(updated);f.flush();os.fsync(f.fileno())
        os.replace(tmp,target)
    finally:
        if Path(tmp).exists():Path(tmp).unlink()
    print(json.dumps({'profileInstalled':PROFILE,'provider':PROVIDER,'defaultRoutingChanged':False}))


if __name__=='__main__':
    os.umask(0o077)
    if sys.argv[1:2]==['token']:
        print(server_record(Path(sys.argv[2]))['token'])
    else:
        p=argparse.ArgumentParser();p.add_argument('--server',type=Path,required=True);p.add_argument('--codex-home',type=Path,required=True);p.add_argument('--catalog-template',type=Path,required=True);a=p.parse_args()
        install(a.server,a.codex_home,a.catalog_template)
