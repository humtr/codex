"""Actual installed TUI/model picker, with a bounded private PTY and no model turn."""
import os,pty,subprocess,fcntl,termios,struct,select,time,signal,json,re,tempfile
from pathlib import Path
with tempfile.TemporaryDirectory(prefix='chatgpt-tui-picker-',dir=os.environ['TMPDIR']) as tmp:
    subprocess.run(['git','init','-q',tmp],check=True,capture_output=True)
    master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',32,100,0,0))
    env=dict(os.environ);env['TERM']='xterm-256color'
    for k in ['TERMUX_SESSION_ID','CODEX_TERMUX_WINDOW_TOKEN','CODEX_TERMUX_WINDOW_READY','CODEX_TERMUX_SESSION_ORIGIN','CODEX_ACCESS_TOKEN','OPENAI_API_KEY','ACCESS_TOKEN']:env.pop(k,None)
    def session():os.setsid();fcntl.ioctl(slave,termios.TIOCSCTTY,0)
    p=subprocess.Popen([os.environ['PREFIX']+'/bin/codex','--profile','chatgpt','--no-alt-screen','-c','features.memories=false'],stdin=slave,stdout=slave,stderr=slave,env=env,cwd=tmp,preexec_fn=session,close_fds=True);os.close(slave)
    def collect(seconds):
        out=b'';end=time.monotonic()+seconds
        while time.monotonic()<end:
            ready,_,_=select.select([master],[],[],.1)
            if ready:
                try:chunk=os.read(master,65536)
                except OSError:break
                for query,response in [(b'\x1b[6n',b'\x1b[1;1R'),(b'\x1b[c',b'\x1b[?1;2c'),(b'\x1b]10;?',b'\x1b]10;rgb:ffff/ffff/ffff\x1b\\'),(b'\x1b]11;?',b'\x1b]11;rgb:0000/0000/0000\x1b\\')]:
                    if query in chunk:os.write(master,response)
                out+=chunk
            if p.poll() is not None:break
        return re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]','',out.decode(errors='replace'))
    try:
        initial=collect(10)
        if 'trust' in initial.lower():os.write(master,b'\r');initial+=collect(5)
        for byte in b'/model':
            os.write(master,bytes([byte]));time.sleep(.1)
        os.write(master,b'\r');menu=collect(3)
        if not re.search('select[^\n]*model|select[^\n]*reasoning',menu,re.I):
            os.write(master,b'\r');menu+=collect(3)
        disabled='disabled until startup' in menu.lower()
        proof={'nativeTuiRunning':p.poll() is None,'pickerOpened':bool(re.search('select[^\n]*model|select[^\n]*reasoning',menu,re.I)) and not disabled,
               'chatgptLabelVisible':'ChatGPT' in menu and 'Instant' in menu,'profileNameVisible':'chatgpt' in initial.lower(),
               'noModelRequestSent':True,'startupIncomplete':disabled,'modelUnavailable':'no models' in menu.lower(),'uiKeywordCounts':{k:menu.lower().count(k.lower()) for k in ['select','model','reasoning','ChatGPT','Instant','all models','esc']}}
        print(json.dumps(proof))
        d=Path.home()/'.local/share/codex-web-experiment/native-provider/tui-proof.json';d.write_text(json.dumps(proof));d.chmod(0o600)
        os.write(master,b'\x1b');collect(.3)
    finally:
        if p.poll() is None:
            os.killpg(p.pid,signal.SIGINT)
            try:p.wait(timeout=3)
            except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGTERM);p.wait(timeout=5)
        os.close(master)
