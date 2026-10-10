"""Actual native FD/pane boundary with a disposable native frontend and socket peer."""
import json
import os
import shlex
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
manager=Path(sys.argv[1]).resolve()
cc=shutil.which('cc') or shutil.which('clang');assert cc
uid=os.getuid();sid='01a0fe94-2d11-79f3-8f5b-9c1958465dc2'
with tempfile.TemporaryDirectory(prefix='co-') as tmp:
    root=Path(tmp);home=root/'h';home.mkdir();prefix=root/'usr';(prefix/'bin').mkdir(parents=True)
    endpoint=root/'apps/com.termux/termux-am/am.sock';endpoint.parent.mkdir(parents=True)
    native=home/'.local/lib/codex/core/generations/owned/helpers/2';native.parent.mkdir(parents=True)
    source=root/'native.c'
    source.write_text(r'''
#define _GNU_SOURCE
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>
static int slot;
static void change(int signal) { (void)signal; lseek(slot,0,SEEK_SET); write(slot,"11a0fe94-2d11-79f3-8f5b-9c1958465dc2",36); }
int main(int argc,char **argv) {
 if(argc!=3)return 10;
 char path[1024],number[32];snprintf(path,sizeof(path),"%s/.codex-terminal-thread-v1-%d-owned",getenv("TMPDIR"),getpid());
 slot=open(path,O_CREAT|O_EXCL|O_RDWR|O_CLOEXEC,0600);if(slot<0)return 11;unlink(path);write(slot,"01a0fe94-2d11-79f3-8f5b-9c1958465dc2",36);
 snprintf(number,sizeof(number),"%d",slot);pid_t child=fork();if(child==0){execl(argv[1],argv[1],"__terminal-bind-v1",number,NULL);_exit(12);}
 int status;waitpid(child,&status,0);if(!WIFEXITED(status)||WEXITSTATUS(status))return 13;
 FILE *ready=fopen(argv[2],"w");fprintf(ready,"%d",getpid());fclose(ready);signal(SIGUSR1,change);for(;;)pause();
}
''')
    subprocess.run([cc,str(source),'-o',str(native)],check=True)
    cli=prefix/'bin/termux-am-socket';cli.write_text('#!'+sys.executable+'\nimport socket,sys\ns=socket.socket(socket.AF_UNIX);s.connect('+repr(str(endpoint))+');s.sendall(sys.argv[1].encode());s.shutdown(socket.SHUT_WR);r=b""\nwhile True:\n d=s.recv(4096)\n if not d:break\n r+=d\nsys.stdout.buffer.write(r)\n');cli.chmod(0o755)
    server=socket.socket(socket.AF_UNIX);server.bind(str(endpoint));server.listen();server.settimeout(.1)
    requests=[];state={'accept':True,'origin':None};stop=threading.Event()
    def serve():
        while not stop.is_set():
            try:client,_=server.accept()
            except socket.timeout:continue
            except OSError:break
            with client:
                request=client.recv(4096).decode();requests.append(request)
                response={'schema':'termux-terminal-origin-v1','result':'ok' if state['accept'] else 'unavailable','origin':state['origin']}
                client.sendall(json.dumps(response).encode())
    thread=threading.Thread(target=serve);thread.start()
    tsock=str(root/'t.sock');ready=root/'ready';env={**os.environ,'HOME':str(home),'PREFIX':str(prefix),'TMPDIR':str(root),'CODEX_TERMUX_CORE_API':'codex-manager-core-v1','CODEX_TERMUX_CORE_ENTRYPOINT':str(manager),'TMUX':'','TMUX_PANE':''}
    def tm(*args):return subprocess.check_output(['tmux','-S',tsock,*args],env=env,text=True).strip()
    def focus(value=sid):return subprocess.run([str(manager),'__terminal-focus-v1',value],env=env,capture_output=True,timeout=8)
    try:
        pane=tm('-f','/dev/null','new-session','-d','-P','-F','#{pane_id}','-s','owned',shlex.join([str(native),str(manager),str(ready)]))
        deadline=time.monotonic()+4
        while not ready.exists() and time.monotonic()<deadline:time.sleep(.01)
        assert ready.exists(),'actual native binding did not publish'
        pid=int(ready.read_text());start=int(Path('/proc',str(pid),'stat').read_text().rsplit(')',1)[1].split()[19])
        origin={'handle':'22a0fe94-2d11-79f3-8f5b-9c1958465dc2','pid':pid,'start':start};state['origin']=origin
        tm('set-option','-t','owned','@termux_origin_v1',json.dumps(origin,separators=(',',':')))
        other=tm('new-window','-d','-P','-F','#{pane_id}','-t','owned','-n','other','sleep 90');tm('select-window','-t',other)
        before=tm('list-panes','-a','-F','#{pane_id}:#{pane_pid}')
        for _ in range(2):
            result=focus();assert result.returncode==0,result.stderr
            assert tm('display-message','-p','-t',pane,'#{window_active}')=='1'
        assert len(requests)==4 and all(r.split()[1] in ('validate','focus') for r in requests)
        assert before==tm('list-panes','-a','-F','#{pane_id}:#{pane_pid}')
        print('PASS actual_native_fd_binding_existing_origin_repeated_focus')
        tm('select-window','-t',other);state['accept']=False
        assert focus().returncode==1
        assert tm('display-message','-p','-t',other,'#{window_active}')=='1'
        assert before==tm('list-panes','-a','-F','#{pane_id}:#{pane_pid}')
        print('PASS unsupported_or_closed_origin_no_selection_or_constructor')
        state['accept']=True;count=len(requests)
        for value in ('bad','$(false)',sid.upper()):assert focus(value).returncode==2
        assert len(requests)==count
        os.kill(pid,10);time.sleep(.02)
        assert focus().returncode==1 and len(requests)==count
        assert tm('display-message','-p','-t',other,'#{window_active}')=='1'
        print('PASS malformed_and_changed_foreground_identity_refused')
    finally:
        subprocess.run(['tmux','-S',tsock,'kill-server'],env=env,capture_output=True)
        stop.set();thread.join(timeout=2);server.close()
