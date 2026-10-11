"""Exercise the actual Manager binary through real disposable PTYs and tmux."""
import fcntl
import json
import os
import pty
import select
import shutil
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path

manager = Path(sys.argv[1]).resolve()
assert shutil.which('tmux')
with tempfile.TemporaryDirectory(prefix='codex-manager-tmux-') as tmp:
    root = Path(tmp); home = root/'home'; home.mkdir(); (root/'run').mkdir()
    marker = root/'runs'; core = root/'core'
    core.write_text('#!'+sys.executable+'\nimport json,os,sys,time\nif sys.argv[1:3]==["termux","__terminal-exec-v1"]:\n os.environ["CODEX_TERMUX_CORE_API"]="codex-manager-core-v1";os.environ["CODEX_TERMUX_CORE_ENTRYPOINT"]=os.path.abspath(sys.argv[0]);os.execv('+repr(str(manager))+',["manager",*sys.argv[2:]])\np='+repr(str(marker))+'\nwith open(p,"a") as f:f.write(json.dumps({"argv":sys.argv[1:],"home":os.environ.get("CODEX_HOME"),"cwd":os.getcwd(),"color":os.environ.get("NO_COLOR"),"tty":os.ttyname(0),"pid":os.getpid()})+"\\n")\nprint("OWNED_WORKLOAD",flush=True)\ntime.sleep(90)\n');core.chmod(0o755)
    env = {**os.environ, 'HOME':str(home), 'CODEX_TERMUX_CORE_API':'codex-manager-core-v1', 'CODEX_TERMUX_CORE_ENTRYPOINT':str(core), 'TMUX':'', 'TMUX_PANE':'', 'TMUX_TMPDIR':str(root/'run'), 'PREFIX':str(root/'prefix'), 'TERM':'xterm-256color'}
    env.pop('CODEX_HOME', None); env.pop('NO_COLOR', None)
    socket = str(root/'run'/('tmux-'+str(os.getuid()))/'default')
    clients=[]; fds=[]
    def tm(*args): return subprocess.check_output(['tmux','-S',socket,*args],text=True).strip()
    def start(args, keys=None):
        master, slave=pty.openpty();fds.extend([master,slave]);fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',12,80,0,0))
        process=subprocess.Popen([str(manager),'tmux',*args],stdin=slave,stdout=slave,stderr=slave,cwd=root,env=env,start_new_session=True);clients.append(process)
        if keys is not None:
            deadline=time.monotonic()+4; output=b''
            while b'Profile (' not in output and process.poll() is None and time.monotonic()<deadline:
                if select.select([master],[],[],.1)[0]:output+=os.read(master,8192)
            assert b'Profile (' in output,output
            os.write(master, keys)
        return process, slave
    def records(count):
        deadline=time.monotonic()+4
        while time.monotonic()<deadline:
            data=[json.loads(line) for line in marker.read_text().splitlines()] if marker.exists() else []
            if len(data)==count:return data
            time.sleep(.02)
        raise AssertionError('missing actual workload: '+str(count))
    try:
        # Non-TTY/invalid inputs must never create a workload.
        for args in ([],['--profile'],['--','update']):
            result=subprocess.run([str(manager),'tmux',*args],env=env,capture_output=True)
            assert result.returncode in (1,2) and not marker.exists()
        print('PASS noninteractive_and_invalid_refusal')
        for keys in (b'\x1b', b'\x03', b'\x1b['):
            process,slave=start([],keys);assert process.wait(timeout=3)==130
            assert termios.tcgetattr(slave)[3] & termios.ICANON
            assert not marker.exists()
        print('PASS picker_cancel_and_partial_escape_restore_tty')
        result=subprocess.run([str(manager),'profile','create','work'],env=env,capture_output=True);assert result.returncode==0,result.stderr
        explicit, explicit_tty=start(['--profile','work','--','resume','space arg',"apostrophe'",'$(false)'])
        first=records(1)[0];assert first['argv']==['resume','space arg',"apostrophe'",'$(false)'] and first['home']==str(home/'.local/share/codex/manager/profiles/work/home');assert first['cwd']==str(root)
        assert explicit.poll() is None and not (root/'false').exists()
        assert tm('show-options','-v','-t','$0','status')=='off'
        assert tm('show-options','-v','-t','$0','set-titles-string')=='#{pane_title}'
        first_state=tm('list-panes','-t','$0','-F','#{pane_id}:#{pane_pid}:#{window_active}')
        print('PASS explicit_profile_exact_argv_current_tty_hidden_title')
        # A stale server color cannot replace the caller's explicit absence.
        tm('set-environment','-g','NO_COLOR','1')
        selected, selected_tty=start([],b'\x1b[B\r');second=records(2)[1]
        assert second['argv']==[] and second['home']==first['home'] and second['color'] is None
        assert tm('list-panes','-t','$0','-F','#{pane_id}:#{pane_pid}:#{window_active}')==first_state
        rows=dict(row.split('\t') for row in tm('list-clients','-F','#{client_tty}\t#{session_id}').splitlines())
        assert rows[os.ttyname(explicit_tty)]=='$0' and rows[os.ttyname(selected_tty)]=='$1'
        print('PASS picker_profile_independent_original_clients_and_color')
        # Inherited external home is available without silently selecting saved/default.
        env['CODEX_HOME']=str(root/'external')
        external,_=start([],b'\r');third=records(3)[2]
        assert third['argv']==[] and third['home']==env['CODEX_HOME']
        print('PASS inherited_external_home_preservation')
        # An inside launch targets source session even while another client is active.
        env['TMUX']=socket+',1,0';env['TMUX_PANE']=tm('list-panes','-t','$0','-F','#{pane_id}')
        before=tm('list-panes','-t','$1','-F','#{pane_id}:#{pane_pid}:#{window_active}')
        inside,_=start(['--profile','work']);assert inside.wait(timeout=3)==0;records(4)
        assert len(tm('list-panes','-s','-t','$0','-F','#{pane_id}').splitlines())==2
        assert tm('list-panes','-t','$1','-F','#{pane_id}:#{pane_pid}:#{window_active}')==before
        print('PASS inside_exact_source_session')
        env['TMUX']='';env['TMUX_PANE']=''
        missing,_=start(['--profile','missing']);assert missing.wait(timeout=3)==1
        assert len(records(4))==4
        print('PASS missing_profile_without_workload')
    finally:
        subprocess.run(['tmux','-S',socket,'kill-server'],capture_output=True)
        for process in clients:
            try:process.wait(timeout=3)
            except subprocess.TimeoutExpired:process.kill();process.wait()
        for fd in fds:os.close(fd)
