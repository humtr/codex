"""Real installed config profile, native commands and final ChatGPT reply."""
import argparse,json,os,signal,subprocess,tempfile
from pathlib import Path

def probe(staged_context=False):
    env=dict(os.environ)
    for key in ['OPENAI_API_KEY','CODEX_ACCESS_TOKEN','ACCESS_TOKEN','TERMUX_SESSION_ID','CODEX_TERMUX_WINDOW_TOKEN','CODEX_TERMUX_WINDOW_READY','CODEX_TERMUX_SESSION_ORIGIN']:
        env.pop(key,None)
    nonce=os.urandom(16).hex()
    with tempfile.TemporaryDirectory(prefix='chatgpt-installed-exec-',dir=os.environ['TMPDIR']) as tmp:
        root=Path(tmp)
        task=f'Use the local shell tool to write exactly {nonce} to proof.txt. Use a separate second shell call to read proof.txt. Reply with exactly {nonce}. Do no other work.'
        if staged_context:task='Reference context for this owned test, retain but take no action yet.\n'+''.join(f'Context row {i}: preserve this reference exactly.\n' for i in range(1200))+'\nActual task:\n'+task
        command=[os.environ['PREFIX']+'/bin/codex','--profile','chatgpt','exec','--ephemeral','--json','--skip-git-repo-check','--sandbox','danger-full-access','-c','approval_policy="on-request"','-c','approvals_reviewer="user"','-c','features.memories=false','-c','web_search="disabled"',task]
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
        file=root/'proof.txt';successful=sum(c.get('exit_code')==0 for c in commands)
        completed=any(r.get('type')=='turn.completed' for r in rows)
        matched=any(reply.strip()==nonce for reply in replies)
        marker=file.is_file() and file.read_text().strip()==nonce
        return {'installedCommandAccepted':process.returncode==0 and successful>=2 and matched and marker and completed,'successfulNativeCommands':successful,'finalReplyMatched':matched,'workspaceMarkerVerified':marker,'turnCompleted':completed,'exit':process.returncode,'stagedContextRequested':staged_context}

if __name__=='__main__':
    os.umask(0o077)
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);p.add_argument('--staged-context',action='store_true');args=p.parse_args()
    report=probe(args.staged_context)
    if args.report.exists() or args.report.is_symlink():
        state=args.report.lstat();assert args.report.is_file() and not args.report.is_symlink() and state.st_uid==os.getuid() and not state.st_mode&0o077
    fd,name=tempfile.mkstemp(dir=args.report.parent,prefix='.native-cli-proof-')
    with os.fdopen(fd,'w') as out:out.write(json.dumps(report)+'\n');out.flush();os.fsync(out.fileno())
    os.replace(name,args.report)
    print(json.dumps(report));raise SystemExit(0 if report['installedCommandAccepted'] else 1)
