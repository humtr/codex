"""Real installed config profile, native commands and final ChatGPT reply."""
import argparse,json,os,signal,subprocess,tempfile
from pathlib import Path

def probe(staged_context=False,tool_search=False,tool_action=False):
    env=dict(os.environ)
    for key in ['OPENAI_API_KEY','CODEX_ACCESS_TOKEN','ACCESS_TOKEN','TERMUX_SESSION_ID','CODEX_TERMUX_WINDOW_TOKEN','CODEX_TERMUX_WINDOW_READY','CODEX_TERMUX_SESSION_ORIGIN']:
        env.pop(key,None)
    nonce=os.urandom(16).hex();native_proof=None;before={}
    if tool_search or tool_action:
        from desktop_provider_profile import server_record
        data=Path.home()/'.local/share/codex-web-experiment'
        binding=server_record(data/'native-provider/server.json')
        session=json.loads((data/'linux-desktop/session.json').read_text());native_proof=Path(session['root'])/'desktop-server.json.proof'
        actual=server_record(Path(session['root'])/'desktop-server.json');assert actual==binding
        before=json.loads(native_proof.read_text())
    with tempfile.TemporaryDirectory(prefix='chatgpt-installed-exec-',dir=os.environ['TMPDIR']) as tmp:
        root=Path(tmp)
        task=f'Use the local shell tool to write exactly {nonce} to proof.txt. Use a separate second shell call to read proof.txt. Reply with exactly {nonce}. Do no other work.'
        if tool_search or tool_action:task=f'Make exactly one external native RPC tool-catalog lookup for GitHub repository metadata, using the advertised catalog-search schema and the bridge tool JSON envelope. This is an external client request, not a desktop app builtin. Wait for the actual native result. Do not execute a discovered tool, use a shell or modify files. Then reply with exactly {nonce}. Do no other work.'
        if tool_action:task=f'This is a live connector qualification, so an answer without the actual native calls is invalid. Your FIRST external RPC response must request the advertised native catalog-search tool; it must not be a final reply. Only after its native result arrives, call the returned read-only metadata tool, and only after that real result arrives may you finish. Make an external native tool-catalog lookup for a read-only GitHub repository metadata tool. From the returned schemas execute that actual native tool to read the public repository humtr/codex. This is an external client request, not a desktop builtin. Do not use a shell, modify files or send messages. Wait for the real tool result, then reply exactly humtr/codex {nonce}. Do no other work.'
        if staged_context:task='Reference context for this owned test, retain but take no action yet.\n'+''.join(f'Context row {i}: preserve this reference exactly.\n' for i in range(1200))+'\nActual task:\n'+task
        command=[os.environ['PREFIX']+'/bin/codex','--profile','chatgpt','exec','--ephemeral','--json','--skip-git-repo-check','--sandbox','danger-full-access','-c','approval_policy="never"','-c','approvals_reviewer="user"','-c','features.memories=false','-c','web_search="disabled"',task]
        with (root/'out').open('w') as out,(root/'err').open('w') as error:
            process=subprocess.Popen(command,cwd=root,env=env,stdin=subprocess.DEVNULL,stdout=out,stderr=error,start_new_session=True)
            try:process.wait(timeout=600)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid,signal.SIGTERM)
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait(timeout=5)
        rows=[]
        for line in (root/'out').read_text().splitlines():
            try:rows.append(json.loads(line))
            except ValueError:pass
        commands=[r['item'] for r in rows if r.get('type')=='item.completed' and r.get('item',{}).get('type')=='command_execution']
        replies=[r['item'].get('text','') for r in rows if r.get('type')=='item.completed' and r.get('item',{}).get('type')=='agent_message']
        searches=[r['item'] for r in rows if r.get('type')=='item.completed' and 'search' in r.get('item',{}).get('type','') and r.get('item',{}).get('type')!='web_search']
        item_types=sorted({r.get('item',{}).get('type') for r in rows if r.get('type')=='item.completed' and r.get('item',{}).get('type')})
        file=root/'proof.txt';successful=sum(c.get('exit_code')==0 for c in commands)
        completed=any(r.get('type')=='turn.completed' for r in rows)
        matched=any(reply.strip()==("humtr/codex "+nonce if tool_action else nonce) for reply in replies)
        marker=file.is_file() and file.read_text().strip()==nonce
        discovered=False;search_results=0
        if tool_search or tool_action:
            after=json.loads(native_proof.read_text());search_results=after.get('returnedNativeSearchResults',0)
            discovered=after.get('modelRequestedSearches',0)>before.get('modelRequestedSearches',0) and search_results>0 and after.get('nativeSearchResultErrors',0)==0 and after.get('nativeSearchLoadedTools',0)>0
        mcps=[r['item'] for r in rows if r.get('type')=='item.completed' and r.get('item',{}).get('type')=='mcp_tool_call']
        successful_mcp=[m for m in mcps if m.get('status')=='completed' and not m.get('error') and m.get('result')]
        public_metadata=any('humtr/codex' in json.dumps(m.get('result',{})) for m in successful_mcp)
        accepted=process.returncode==0 and matched and completed and (discovered and successful_mcp and public_metadata and not commands if tool_action else discovered and not commands if tool_search else successful>=2 and marker)
        return {'installedCommandAccepted':bool(accepted),'nativeToolSearchSeen':discovered or bool(searches),'returnedNativeSearchResults':search_results,'completedItemTypes':item_types,'toolSearchRequested':tool_search or tool_action,'actualDiscoveredToolRequested':tool_action,'successfulDiscoveredCalls':len(successful_mcp),'publicMetadataVerified':public_metadata,'successfulNativeCommands':successful,'finalReplyMatched':matched,'workspaceMarkerVerified':marker,'turnCompleted':completed,'exit':process.returncode,'stagedContextRequested':staged_context}

if __name__=='__main__':
    os.umask(0o077)
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);p.add_argument('--staged-context',action='store_true');p.add_argument('--tool-search',action='store_true');p.add_argument('--tool-action',action='store_true');args=p.parse_args();assert sum(bool(x) for x in [args.staged_context,args.tool_search,args.tool_action])<=1
    report=probe(args.staged_context,args.tool_search,args.tool_action)
    if args.report.exists() or args.report.is_symlink():
        state=args.report.lstat();assert args.report.is_file() and not args.report.is_symlink() and state.st_uid==os.getuid() and not state.st_mode&0o077
    fd,name=tempfile.mkstemp(dir=args.report.parent,prefix='.native-cli-proof-')
    with os.fdopen(fd,'w') as out:out.write(json.dumps(report)+'\n');out.flush();os.fsync(out.fileno())
    os.replace(name,args.report)
    print(json.dumps(report));raise SystemExit(0 if report['installedCommandAccepted'] else 1)
