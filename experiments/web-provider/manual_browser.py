"""Ordinary persistent Chromium for human login/challenge troubleshooting only."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from fit_browser import fit_browser

root,candidate,bun,state = (Path(value).resolve(strict=True) for value in sys.argv[1:])
profile=state.parent/'manual-profile'
profile.mkdir(mode=0o700,exist_ok=True)
assert not profile.is_symlink() and profile.stat().st_mode&0o777==0o700
assert json.loads(Path(str(state)+'.verified.json').read_text()).get('authenticated') is True
env=json.loads((root/'environment.json').read_text())
metadata=root/'account-view-process.json'
if metadata.exists():
    old=json.loads(metadata.read_text());proc=Path('/proc')/str(old['pid'])
    if proc.exists() and proc.joinpath('stat').read_text().rsplit(')',1)[1].split()[0]!='Z' and proc.joinpath('stat').read_text().rsplit(')',1)[1].split()[19]==str(old['start']):
        raise RuntimeError('Existing owned account browser must finish before replacement')
seed=subprocess.run([str(bun),str(Path(__file__).with_name('seed_manual_profile.mjs')),str(candidate),str(profile),str(state)],env=env,capture_output=True,timeout=60)
if seed.returncode:
    raise RuntimeError('Private account profile initialization failed; credentials were not logged')
# Standard Chromium launch: no Playwright driver or debugging endpoint is attached.
browser=subprocess.Popen([env['PREFIX']+'/lib/chromium/chrome','--user-data-dir='+str(profile),'--no-sandbox','--disable-dev-shm-usage','--disable-background-mode','--no-first-run','--no-default-browser-check','--window-position=0,0','--new-window','https://chatgpt.com/?temporary-chat=true'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
browser_proc=Path('/proc')/str(browser.pid)
metadata.write_text(json.dumps({'pid':browser.pid,'image':os.readlink(browser_proc/'exe'),'start':browser_proc.joinpath('stat').read_text().rsplit(')',1)[1].split()[19]}));metadata.chmod(0o600)
def stop(*_):
    if browser.poll() is None:
        browser.terminate()
signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
try:
    for _ in range(100):
        if browser.poll() is not None:
            raise RuntimeError('Ordinary browser exited during startup')
        try:
            fit_browser(root)
            (root/'account-view-ready.json').write_text(json.dumps({'pid':browser.pid,'opened':True,'ordinaryPersistentBrowser':True,'time':time.time()}))
            break
        except RuntimeError:
            time.sleep(.1)
    else:
        raise RuntimeError('Ordinary browser window was not ready')
    raise SystemExit(browser.wait())
finally:
    if browser.poll() is None:
        browser.terminate()
        try:browser.wait(timeout=5)
        except subprocess.TimeoutExpired:browser.kill();browser.wait()
