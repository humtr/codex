"""Real Manager, native foreground FD and PTYs; both Android protocol routes."""
import fcntl
import json
import os
import pty
import select
import shlex
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from pathlib import Path

manager = Path(sys.argv[1]).resolve()
sid1='01a0fe94-2d11-79f3-8f5b-9c1958465dc2'
sid2='11a0fe94-2d11-79f3-8f5b-9c1958465dc2'
with tempfile.TemporaryDirectory(prefix='codex-window-') as tmp:
    root=Path(tmp);home=root/'h';home.mkdir();prefix=root/'usr';(prefix/'bin').mkdir(parents=True)
    endpoint=root/'apps/com.termux/termux-am/am.sock';endpoint.parent.mkdir(parents=True)
    native=home/'.local/lib/codex/core/generations/owned/helpers/2';native.parent.mkdir(parents=True)
    source=root/'native.c';source.write_text(r'''
#define _GNU_SOURCE
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>
static int slot;
static void change(int sig) { (void)sig; lseek(slot,0,SEEK_SET); write(slot,"11a0fe94-2d11-79f3-8f5b-9c1958465dc2",36); }
int main(int argc,char **argv) {
 char path[1024],number[32];snprintf(path,sizeof(path),"%s/.codex-terminal-thread-v1-%d-owned",getenv("TMPDIR"),getpid());
 slot=open(path,O_CREAT|O_EXCL|O_RDWR|O_CLOEXEC,0600);if(slot<0)return 11;unlink(path);write(slot,getenv("FIXTURE_SID"),36);
 snprintf(number,sizeof(number),"%d",slot);pid_t child=fork();if(child==0){execl(getenv("FIXTURE_MANAGER"),"manager","__terminal-bind-v1",number,NULL);_exit(12);}
 int status;waitpid(child,&status,0);if(!WIFEXITED(status)||WEXITSTATUS(status))return 13;
 snprintf(path,sizeof(path),"%s/%d.ready",getenv("TMPDIR"),getpid());FILE *ready=fopen(path,"w");fprintf(ready,"%s",ttyname(0));fclose(ready);
 signal(SIGUSR1,change);for(;;)pause();
}
''')
    subprocess.run([shutil.which('cc') or 'clang',str(source),'-o',str(native)],check=True)
    core=root/'core';log=root/'workloads'
    core.write_text('#!'+sys.executable+'\n'+'''
import json,os,sys
from pathlib import Path
manager='''+repr(str(manager))+'''
os.environ['CODEX_TERMUX_CORE_API']='codex-manager-core-v1'
os.environ['CODEX_TERMUX_CORE_ENTRYPOINT']=os.path.abspath(sys.argv[0])
if sys.argv[1:3] in (['termux','__terminal-start-v1'],['termux','__terminal-exec-v1']):
 os.execv(manager,[manager,*sys.argv[2:]])
if sys.argv[1:4]==['termux','profile','use']:
 name=sys.argv[4];os.environ['CODEX_HOME']=str(Path.home()/'.codex' if name=='default' else Path.home()/'.local/share/codex/manager/profiles'/name/'home');args=sys.argv[6:]
else:args=sys.argv[1:]
os.environ['FIXTURE_SID']=next((arg for arg in args if len(arg)==36),os.environ.get('FIXTURE_SID'))
with open('''+repr(str(log))+''','a') as f:f.write(json.dumps({'argv':[list(os.fsencode(a)) for a in args],'pid':os.getpid(),'tty':os.ttyname(0),'home':os.environ.get('CODEX_HOME'),'color':os.environ.get('NO_COLOR'),'cwd':os.getcwd(),'tmux':bool(os.environ.get('TMUX')),'auth':os.environ.get('OWNED_PRIVATE_AUTH')})+'\\n')
os.execv('''+repr(str(native))+''',['native'])
''');core.chmod(0o755)
    cli=prefix/'bin/termux-am-socket';cli.write_text('#!'+sys.executable+'\nimport socket,sys\ns=socket.socket(socket.AF_UNIX);s.connect('+repr(str(endpoint))+');s.sendall(sys.argv[1].encode());s.shutdown(socket.SHUT_WR);r=b""\nwhile True:\n d=s.recv(4096)\n if not d:break\n r+=d\no=json.loads(r) if False else None\nsys.stdout.buffer.write(r)\n');cli.chmod(0o755)
    # Client emits the native CLI's stdout/stderr/exit status, not wire framing.
    text=cli.read_text().replace('o=json.loads(r) if False else None\nsys.stdout.buffer.write(r)', 'import json\no=json.loads(r);sys.stdout.write(o["stdout"]);sys.stderr.write(o["stderr"]);sys.exit(o["status"])');cli.write_text(text)
    (prefix/'bin/true').symlink_to(shutil.which('true'))
    (prefix/'bin/env').symlink_to(shutil.which('env'))
    env={**os.environ,'HOME':str(home),'PREFIX':str(prefix),'TMPDIR':str(root),'TMUX_TMPDIR':str(root), 'TMUX':'','TMUX_PANE':'','TERM':'xterm-256color','CODEX_TERMUX_CORE_API':'codex-manager-core-v1','CODEX_TERMUX_CORE_ENTRYPOINT':str(core),'FIXTURE_MANAGER':str(manager),'FIXTURE_SID':sid1,'OWNED_PRIVATE_AUTH':'synthetic-auth-retained'}
    for key in ('CODEX_HOME','NO_COLOR','CODEX_TERMUX_WINDOW_TOKEN','CODEX_TERMUX_WINDOW_READY'):env.pop(key,None)
    server=socket.socket(socket.AF_UNIX);server.bind(str(endpoint));server.listen();server.settimeout(.1)
    state={'mode':'native','windows':{},'requests':[],'errors':[]};stop=threading.Event();processes=[];fds=[]
    def start(command,environment=env,cwd=root):
        master,slave=pty.openpty();fds.extend([master,slave]);fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',24,100,0,0))
        process=subprocess.Popen(command,stdin=slave,stdout=slave,stderr=slave,env=environment,cwd=cwd,start_new_session=True);processes.append(process)
        return process,master,slave
    def identity(pid):return int(Path('/proc',str(pid),'stat').read_text().rsplit(')',1)[1].split()[19])
    def serve():
        while not stop.is_set():
            try:client,_=server.accept()
            except socket.timeout:continue
            except OSError:break
            with client:
                try:
                    peer=struct.unpack('3i',client.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))[0]
                    request=client.recv(4096).decode();state['requests'].append(request);words=shlex.split(request)
                    result={'status':0,'stderr':'','stdout':''}
                    if words[0]=='termux-terminal-v1':
                        if state['mode']=='stock':result.update(stderr="usage: am\n"+" bounded usage line\n"*300+"Error: unknown command 'termux-terminal-v1'\n",status=1)
                        elif state['mode']=='unavailable':result.update(stderr='terminal control unavailable\n',status=69)
                        else:
                            op=words[1]
                            if op=='capabilities':value={'schema':'termux-terminal-origin-v1','result':'ok','operations':'origin,validate,focus'}
                            else:
                                if op=='origin':
                                    pid=int(Path('/proc',str(peer),'stat').read_text().rsplit(')',1)[1].split()[1])
                                    if b'__terminal-bind-v1' in Path('/proc',str(pid),'cmdline').read_bytes():pid=int(Path('/proc',str(pid),'stat').read_text().rsplit(')',1)[1].split()[1])
                                    origin={'handle':f'{pid:08x}-2d11-79f3-8f5b-9c1958465dc2','pid':pid,'start':identity(pid)}
                                else:origin={'handle':words[2],'pid':int(words[3]),'start':int(words[4])}
                                live=identity(origin['pid'])==origin['start']
                                value={'schema':'termux-terminal-origin-v1','result':'ok' if live else 'unavailable','origin':origin}
                            result['stdout']=json.dumps(value)
                    elif words[0]=='startservice':
                        def extra(key):return words[words.index('com.termux.RUN_COMMAND_'+key)+1]
                        name=extra('SHELL_NAME');program=extra('PATH');arguments=extra('ARGUMENTS').split(',') if extra('ARGUMENTS') else []
                        if name not in state['windows']:
                            process,_,_=start([program,*arguments],environment={**env,'HOME':str(root/'service-home'),'OWNED_PRIVATE_AUTH':'wrong-service-value','NO_COLOR':'wrong-service-value'},cwd=extra('WORKDIR'));state['windows'][name]=process
                        result['stdout']='Starting service: Intent {}\n'
                    else:raise AssertionError(request)
                    client.sendall(json.dumps(result).encode())
                except Exception as error:state['errors'].append(repr(error))
    thread=threading.Thread(target=serve);thread.start()
    def run(*args):return subprocess.run([str(manager),*args],env=env,capture_output=True,timeout=10)
    def focus(sid):return run('__terminal-focus-v1',sid)
    def workloads(count):
        deadline=time.monotonic()+8
        while time.monotonic()<deadline:
            rows=[json.loads(v) for v in log.read_text().splitlines()] if log.exists() else []
            if len(rows)>=count and all((root/f'{r["pid"]}.ready').exists() for r in rows):return rows
            time.sleep(.02)
        raise AssertionError(('workloads',count,rows,state['errors']))
    def calls():return len([r for r in state['requests'] if r.startswith('startservice')])
    tsock=str(root/('tmux-'+str(os.getuid()))/'default')
    try:
        a,_,a_tty=start([str(manager),'launch','--profile','default','--',sid1]);r1=workloads(1)[0]
        b,_,b_tty=start([str(manager),'launch','--profile','default','--',sid2]);r2=workloads(2)[1]
        assert r1['tty']==os.ttyname(a_tty) and r2['tty']==os.ttyname(b_tty) and not r1['tmux'] and not r2['tmux'] and calls()==0
        first=focus(sid1);second=focus(sid2);assert first.returncode==0 and second.returncode==0 and calls()==0,(first.stderr,second.stderr,state['requests'])
        print('PASS bare_independent_invoking_ptys_existing_native_return')
        os.kill(r1['pid'],signal.SIGUSR1);time.sleep(.05);before=len(state['requests'])
        assert focus(sid1).returncode==1 and focus(sid2).returncode==1 and len(state['requests'])==before
        os.kill(r1['pid'],signal.SIGTERM);a.wait(timeout=3);assert focus(sid2).returncode==0
        print('PASS thread_switch_ambiguity_and_dead_process_pruning')
        os.kill(r2['pid'],signal.SIGTERM);b.wait(timeout=3)
        assert focus(sid2).returncode==1
        state['mode']='stock';before=calls()
        launch,_,_=start([str(manager),'launch','--profile','default','--',sid1,os.fsdecode(b'raw-\xff'),"quote' $(false)"])
        assert launch.wait(timeout=8)==0
        r3=workloads(3)[2];assert not r3['tmux'] and r3['argv']==[list(os.fsencode(x)) for x in [sid1,os.fsdecode(b'raw-\xff'),"quote' $(false)"]]
        assert r3['home']==str(home/'.codex') and r3['color'] is None and r3['auth']=='synthetic-auth-retained' and r3['cwd']==str(root) and calls()==before+1
        for _ in range(2):assert focus(sid1).returncode==0
        assert len(workloads(3))==3 and len(state['windows'])==1 and calls()==before+3
        assert all(shlex.split(r)[shlex.split(r).index('com.termux.RUN_COMMAND_PATH')+1]==str(prefix/'bin/true') for r in state['requests'] if r.startswith('startservice') and '__terminal-start-v1' not in r)
        assert not list((home/'.local/share/codex/manager/terminals').glob('launch-*')) and not list((home/'.local/share/codex/manager/terminals').glob('consumed-*'))
        print('PASS stock_bare_one_launch_raw_argv_profile_and_inert_reuse')
        os.kill(r3['pid'],signal.SIGTERM);time.sleep(.05);before=calls();assert focus(sid1).returncode==1 and calls()==before
        print('PASS closed_stock_workload_never_restarted')
        launch,_,_=start([str(manager),'__terminal-launch-command-v1','off','--',str(core),sid1,'private-plan-'+'x'*2000]);assert launch.wait(timeout=8)==0
        r4=workloads(4)[3];assert not r4['tmux'] and len(r4['argv'][-1])==2013
        assert focus(sid1).returncode==0
        os.kill(r4['pid'],signal.SIGTERM);time.sleep(.05)
        print('PASS generic_client_one_shot_large_private_execution_plan')

        launch,_,_=start([str(manager),'launch','--profile','default','--tmux=status','--',sid2]);assert launch.wait(timeout=8)==0
        r5=workloads(5)[4];assert r5['tmux'] and r5['auth']=='synthetic-auth-retained' and len(state['windows'])==3
        assert subprocess.check_output(['tmux','-S',tsock,'show-options','-v','-t','$0','status'],text=True).strip()=='on'
        assert focus(sid2).returncode==0 and len(workloads(5))==5
        print('PASS stock_tmux_status_separate_window_existing_pane_return')
        rejected,_,_=start([str(manager),'attach','0']);assert rejected.wait(timeout=3)==1
        client=next(p for p in state['windows'].values() if p.poll() is None)
        client.terminate();client.wait(timeout=3);time.sleep(.1)
        assert Path('/proc',str(r5['pid'])).exists()
        attach,_,_=start([str(manager),'attach','0']);assert attach.wait(timeout=8)==0
        assert focus(sid2).returncode==0 and len(workloads(5))==5 and len(state['windows'])==4
        print('PASS detached_work_rebinds_new_terminal_without_relaunch')

        subprocess.run(['tmux','-S',tsock,'kill-server'],capture_output=True)
        time.sleep(.1);state['mode']='unavailable';before=calls()
        launch,_,_=start([str(manager),'launch','--profile','default']);assert launch.wait(timeout=8)==13
        assert calls()==before and len(state['windows'])==4
        print('PASS inconclusive_native_failure_no_constructor_fallback')
        assert not state['errors'],state['errors']
    finally:
        subprocess.run(['tmux','-S',tsock,'kill-server'],capture_output=True)
        for process in processes:
            if process.poll() is None:
                try:os.killpg(process.pid,signal.SIGTERM)
                except ProcessLookupError:pass
            try:process.wait(timeout=2)
            except subprocess.TimeoutExpired:process.kill();process.wait()
        stop.set();thread.join(timeout=2);server.close()
        for fd in fds:os.close(fd)
