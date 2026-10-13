"""Install the explicitly opted-in desktop trial from verified local dependencies."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
from desktop_lifecycle import DATA,RUNTIME,endpoint,ensure
from open_dual_trial import record
from desktop_provider_profile import install,server_record
from linux_desktop_trial import PROOT_SOURCE,private,write

SOURCE=Path(__file__).resolve().parent


def hashes(root):
    return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.rglob('*')) if p.is_file() and '__pycache__' not in p.parts}


def tree_copy(source,target):
    assert source.is_dir() and not source.is_symlink()
    # Dependencies are immutable source/assets, never account data.
    for p in source.rglob('*'):
        assert not p.is_symlink(), 'symlink_in_dependency'
    shutil.copytree(source,target,ignore=shutil.ignore_patterns('.git','node_modules','__pycache__'))


def build(args):
    dependencies=[args.bridge,args.proot,args.novnc,args.bun]
    assert all(dependencies) or not any(dependencies), 'supply all dependencies or reuse the installed payload'
    installed=None
    if not any(dependencies):
        installed=Path(record(RUNTIME)['installation'])
        accepted=record(installed/'.installation.json')
        expected={**accepted['files'],'.installation.json':hashlib.sha256((json.dumps(accepted)+'\n').encode()).hexdigest()}
        assert hashes(installed)==expected,'installed dependency changed'
        commit=accepted['bridgeCommit']
    else:
        commit=subprocess.check_output(['git','-C',str(args.bridge),'rev-parse','HEAD'],text=True).strip()
        assert commit=='92a356fac2292e3af5a97ab7ba634edd8d38621e'
        assert subprocess.check_output(['git','-C',str(args.bridge),'diff','--binary'])==(SOURCE/'desktop-bridge-615.patch').read_bytes(), 'qualified bridge patch changed'
        assert subprocess.check_output(['git','-C',str(args.proot),'rev-parse','HEAD'],text=True).strip()==PROOT_SOURCE
        assert not subprocess.check_output(['git','-C',str(args.proot),'status','--porcelain'],text=True).strip()
    install_root=private(Path.home()/'.local/lib/codex-web-experiment')
    with tempfile.TemporaryDirectory(prefix='provider-build-',dir=os.environ['TMPDIR']) as tmp:
        stage=Path(tmp)/'payload';stage.mkdir(mode=0o700)
        tree_copy(SOURCE,stage/'source')
        if installed:
            for name in ['bun','bridge.mjs']:
                shutil.copyfile(installed/name,stage/name)
                shutil.copymode(installed/name,stage/name)
            for name in ['novnc','proot','licenses','bridge-source']:tree_copy(installed/name,stage/name)
        else:
            tree_copy(args.novnc,stage/'novnc')
            (stage/'proot').mkdir(mode=0o700)
            archive=subprocess.check_output(['git','-C',str(args.proot),'archive',PROOT_SOURCE])
            with tarfile.open(fileobj=io.BytesIO(archive)) as t:t.extractall(stage/'proot',filter='data')
            write(stage/'proot/.codex-provider-source.json',{'commit':PROOT_SOURCE,'files':hashes(stage/'proot')})
            shutil.copyfile(args.bun,stage/'bun');(stage/'bun').chmod(0o700)
            exports=[('parseRequest','responses/parser.ts'),('AsyncEventQueue','event-queue.ts'),('bridgeToResponsesSSE,formatErrorResponse','bridge.ts'),('namespacedToolName','types.ts'),('extractCodexTurnIdentityFromBody','adapters/chatgpt-web/environment.ts'),('SUMMARY_PREFIX','responses/compaction.ts')]
            entry=Path(tmp)/'entry.ts';entry.write_text('\n'.join('export {'+names+'} from '+json.dumps(str(args.bridge/'src'/file))+';' for names,file in exports))
            subprocess.run([str(args.bun),'build','--target=bun',str(entry),'--outfile',str(stage/'bridge.mjs')],check=True,capture_output=True)
            licenses=stage/'licenses';licenses.mkdir(mode=0o700)
            shutil.copytree(args.bridge/'LICENSES',licenses/'bridge-LICENSES')
            # Preserve the effective bridge source without caches, auth logs or dependencies.
            tree_copy(args.bridge/'src',stage/'bridge-source')
            for name,root in [('bridge',args.bridge),('proot',args.proot),('novnc',args.novnc)]:
                for p in root.glob('*LICENSE*'):
                    if p.is_file():shutil.copyfile(p,licenses/(name+'-'+p.name))
        try:
            source_head=subprocess.check_output(['git','-C',str(SOURCE),'rev-parse','HEAD'],text=True,stderr=subprocess.DEVNULL).strip()
        except subprocess.CalledProcessError:
            # The installed script is also a valid reinstall entrypoint after TMPDIR is gone.
            source_head=record(SOURCE.parent/'.installation.json')['sourceBase']
        manifest={'bridgeCommit':commit,'prootCommit':PROOT_SOURCE,'sourceBase':source_head,'files':hashes(stage)}
        digest=hashlib.sha256(json.dumps(manifest,sort_keys=True).encode()).hexdigest()
        target=install_root/digest[:20]
        if target.exists():
            assert hashes(target)=={**manifest['files'],'.installation.json':hashlib.sha256((json.dumps(manifest)+'\n').encode()).hexdigest()}, 'existing_installation_changed'
        else:
            # Copy and then rename on the final filesystem: Android hardlinks unsupported.
            partial=Path(tempfile.mkdtemp(prefix='.install-',dir=install_root))
            try:
                shutil.copytree(stage,partial,dirs_exist_ok=True)
                write(partial/'.installation.json',manifest)
                os.rename(partial,target)
            finally:
                if partial.exists():shutil.rmtree(partial) # Owned generated installation only.
        return target,manifest


def activate(args):
    if RUNTIME.exists() or RUNTIME.is_symlink():ensure(RUNTIME)
    previous=server_record(DATA/'server.json')
    endpoint(previous)
    for home in args.home:
        target=home/'chatgpt.config.toml'
        if target.exists():
            from desktop_provider_profile import BEGIN
            assert target.read_text().startswith(BEGIN), 'existing_user_profile_conflict'
    target,manifest=build(args)
    runtime={k:previous[k] for k in ['baseUrl','token','model']}
    runtime.update({'source':str(target/'source'),'bun':str(target/'bun'),'bridge':str(target/'bridge.mjs'),'proot':str(target/'proot'),'novnc':str(target/'novnc'),
                    'session':str(Path.home()/'.local/share/codex-web-experiment/linux-desktop/session.json'),'binding':str(DATA/'server.json'),'installation':str(target), 'production':False})
    if RUNTIME.exists():
        old=record(RUNTIME);assert all(old[k]==runtime[k] for k in ['baseUrl','token','model'])
    # Do not stop processes here. Migration/qualification must settle native jobs first.
    write(RUNTIME,runtime)
    for home in args.home:install(DATA/'server.json',home,args.catalog,RUNTIME)
    print(json.dumps({'installed':True,'permanentPaths':True,'bridgeBundled':True,'production':False,'installation':target.name}))


if __name__=='__main__':
    os.umask(0o077)
    p=argparse.ArgumentParser();p.add_argument('--bridge',type=Path);p.add_argument('--proot',type=Path);p.add_argument('--novnc',type=Path);p.add_argument('--bun',type=Path);p.add_argument('--catalog',type=Path,default=DATA/'models.json');p.add_argument('--home',type=Path,action='append',required=True);a=p.parse_args()
    activate(a)
