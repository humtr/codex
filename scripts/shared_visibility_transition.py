#!/usr/bin/env python3
"""One-shot, user-authorized seven-day transition; no auth copying or SQLite writes."""
from __future__ import annotations
import argparse
import collections
import contextlib
import ctypes
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import sqlite3
import stat
import time
import uuid

DIRECTORIES = ('sessions', 'archived_sessions', 'thread-writer-locks', 'rollout-migrations', 'memories', 'memories_v2', 'tui-thread-reference-capabilities')
FILES = ('session_index.jsonl', 'history.jsonl', 'installation_id')
MAX_ENTRIES = 16384

class TransitionError(RuntimeError):
    pass

def private(path: Path) -> None:
    if path.is_symlink(): raise TransitionError('substituted directory')
    path.mkdir(mode=0o700, exist_ok=True)
    m = path.stat()
    if not stat.S_ISDIR(m.st_mode) or m.st_uid != os.getuid() or stat.S_IMODE(m.st_mode) != 0o700:
        raise TransitionError('unsafe private directory')

def roots(home: Path) -> list[Path]:
    if not home.is_absolute() or home.resolve() != home: raise TransitionError('unsafe home')
    legacy = home / '.codex-profiles'
    if legacy.is_symlink(): raise TransitionError('substituted legacy root')
    result = [home / '.codex']
    if legacy.exists():
        for root in sorted(legacy.iterdir()):
            if root.is_symlink() or not root.is_dir(): raise TransitionError('unsafe legacy identity')
            if not root.name or len(root.name)>64 or not all(c.isascii() and (c.isalnum() or c in '-_') for c in root.name):
                raise TransitionError('invalid legacy identity')
            result.append(root)
    return result

def state(root: Path) -> dict[str, dict]:
    path = root / 'state_5.sqlite'
    if not path.exists(): return {}
    if path.is_symlink(): raise TransitionError('substituted legacy projection')
    con = sqlite3.connect(f'file:{path}?mode=ro', uri=True)
    con.row_factory = sqlite3.Row
    try:
        columns = {row[1] for row in con.execute('pragma table_info(threads)')}
        activity = [c for c in ('recency_at_ms','updated_at_ms','recency_at','updated_at','created_at_ms','created_at') if c in columns]
        if not activity or 'id' not in columns: raise TransitionError('unsupported activity projection')
        extra = [c for c in ('name','project_id','daybreak_enabled','thread_section_id') if c in columns]
        rows = {}
        for row in con.execute('select '+','.join(['id']+activity+extra)+' from threads'):
            value = dict(row)
            value['activity'] = max((float(value[c]) / (1000 if c.endswith('_ms') else 1) for c in activity if isinstance(value[c], (int,float))), default=0)
            rows[value['id']] = value
        # Retained metadata with non-reconstructible thread dependencies must not vanish.
        tables = {row[0] for row in con.execute("select name from sqlite_master where type='table'")}
        for table in ('thread_spawn_edges','thread_dynamic_tools','thread_attachments','thread_artifacts'):
            if table in tables:
                for row in con.execute('select * from '+table):
                    cols = row.keys()
                    ids = [row[c] for c in ('thread_id','parent_thread_id','child_thread_id') if c in cols]
                    for identity in ids:
                        if identity in rows: rows[identity]['dependency'] = True
        return rows
    finally: con.close()

def rollout(path: Path) -> dict:
    series = collections.defaultdict(list)
    last = 0.0
    identity = None
    digest = hashlib.sha256()
    with path.open('rb') as handle:
        for raw in handle:
            if len(raw)>4*1024*1024: raise TransitionError('rollout record exceeds bound')
            digest.update(raw)
            try: obj = json.loads(raw)
            except (ValueError, TypeError): raise TransitionError('invalid or concurrently incomplete rollout') from None
            if not isinstance(obj,dict): raise TransitionError('invalid rollout record')
            if obj.get('type') == 'session_meta':
                identity = str(uuid.UUID(obj['payload']['id']))
            timestamp = obj.get('timestamp')
            if isinstance(timestamp,str):
                try: last=max(last, dt.datetime.fromisoformat(timestamp.replace('Z','+00:00')).timestamp())
                except ValueError: raise TransitionError('invalid activity timestamp') from None
            series[obj.get('type','')].append(hashlib.sha256(json.dumps(obj.get('payload'),sort_keys=True,separators=(',',':')).encode()).digest())
    if not identity: raise TransitionError('missing thread identity')
    m=path.stat()
    return dict(id=identity,path=path,series=dict(series),last=last,digest=digest.hexdigest(),inode=(m.st_dev,m.st_ino))

def inspect(home: Path, cutoff: float) -> dict:
    stores=roots(home)
    records=collections.defaultdict(list)
    metadata={}
    count=0
    legacy=False
    for root in stores:
        metadata[root]=state(root)
        for directory in ('sessions','archived_sessions'):
            base=root/directory
            if base.is_symlink():
                if base.readlink()!=stores[0]/directory: raise TransitionError('unexpected session directory alias')
                continue
            if not base.exists(): continue
            if root!=stores[0]: legacy=True
            for parent, dirs, files in os.walk(base, followlinks=False):
                parent=Path(parent)
                if any((parent/d).is_symlink() for d in dirs): raise TransitionError('session directory symlink')
                for name in files:
                    count+=1
                    if count>MAX_ENTRIES: raise TransitionError('session inventory exceeds bound')
                    path=parent/name
                    if path.suffix!='.jsonl': raise TransitionError('unexpected session payload')
                    if path.is_symlink():
                        if not path.exists(): continue
                        target=path.resolve()
                        if not any(target.is_relative_to(r/'sessions') or target.is_relative_to(r/'archived_sessions') for r in stores): raise TransitionError('rollout alias escapes declared stores')
                        continue  # Every valid alias target is inventoried at its real store.
                    if not path.is_file(): raise TransitionError('nonregular session payload')
                    entry=rollout(path);entry['root']=root;entry['relative']=path.relative_to(root)
                    meta=metadata[root].get(entry['id'],{})
                    entry['last']=max(entry['last'],meta.get('activity',0))
                    records[entry['id']].append(entry)
    selected={}
    for identity, copies in records.items():
        if max(c['last'] for c in copies)<cutoff: continue
        winner=max(copies,key=lambda c:(sum(map(len,c['series'].values())),c['last'],str(c['path'])))
        for copy in copies:
            if any(winner['series'].get(kind,[])[:len(sequence)]!=sequence for kind,sequence in copy['series'].items()): raise TransitionError('divergent retained thread history')
            meta=metadata[copy['root']].get(identity,{})
            if meta.get('dependency') or meta.get('project_id') or meta.get('daybreak_enabled'):
                raise TransitionError('retained thread has unsupported dependency metadata')
        selected[identity]=winner
    return dict(stores=stores,records=dict(records),selected=selected,metadata=metadata,legacy=legacy,cutoff=cutoff)

def writers(snapshot: dict, proc: Path=Path('/proc')) -> tuple[set[str],set[Path]]:
    by_inode={c['inode']:identity for identity,copies in snapshot['records'].items() for c in copies}
    active=collections.defaultdict(set);databases=set()
    for process in proc.iterdir():
        if not process.name.isdigit() or int(process.name)==os.getpid(): continue
        try:
            executable=os.readlink(process/'exe').removesuffix(' (deleted)')
            if not executable.endswith('/runtime'): continue
            for fd in (process/'fd').iterdir():
                try:
                    m=fd.stat();inode=(m.st_dev,m.st_ino)
                    if inode in by_inode: active[by_inode[inode]].add(inode)
                    path=Path(os.readlink(fd).removesuffix(' (deleted)'))
                    for root in snapshot['stores']:
                        if path.parent==root and path.name.startswith(('state_','thread_history_')): databases.add(root)
                except (OSError,ValueError): continue
        except OSError: continue
    for identity, inodes in active.items():
        chosen=snapshot['selected'].get(identity)
        if chosen is None or inodes != {chosen['inode']}:
            raise TransitionError('active excluded or nonmaximal rollout prevents handoff')
    return set(active),databases

def exchange(left: Path, right: Path) -> None:
    number={'aarch64':276,'x86_64':316}.get(platform.machine())
    if number is None: raise TransitionError('atomic exchange unsupported on this architecture')
    libc=ctypes.CDLL(None,use_errno=True)
    result=libc.syscall(ctypes.c_long(number),ctypes.c_int(-100),ctypes.c_char_p(os.fsencode(left)),ctypes.c_int(-100),ctypes.c_char_p(os.fsencode(right)),ctypes.c_uint(2))
    if result!=0: raise OSError(ctypes.get_errno(),'atomic profile exchange failed')

def alias(path: Path, target: Path) -> None:
    if path.is_symlink() and path.readlink()==target: return
    staging=path.with_name(f'.visibility-{os.getpid()}-{path.name}')
    if staging.exists() or staging.is_symlink(): raise TransitionError('transition staging collision')
    staging.symlink_to(target)
    if path.exists() or path.is_symlink():
        exchange(path,staging)
        if staging.is_dir() and not staging.is_symlink(): shutil.rmtree(staging)
        else: staging.unlink()
    else: staging.rename(path)
    fd=os.open(path.parent,os.O_RDONLY)
    try: os.fsync(fd)
    finally: os.close(fd)

def append_shared_records(snapshot: dict, name: str) -> None:
    target=snapshot['stores'][0]/name
    identity_key='session_id' if name=='history.jsonl' else 'id'
    seen=set();append=[]
    for root in snapshot['stores']:
        path=root/name
        if path.is_symlink():
            if path.readlink()!=target: raise TransitionError('unexpected history alias')
            continue
        if not path.exists(): continue
        with path.open('rb') as handle:
            for raw in handle:
                if len(raw)>4*1024*1024: raise TransitionError('history record exceeds bound')
                try: obj=json.loads(raw)
                except ValueError: raise TransitionError('invalid history record') from None
                normalized=json.dumps(obj,sort_keys=True,separators=(',',':')).encode()
                if root==snapshot['stores'][0]: seen.add(normalized)
                elif obj.get(identity_key) in snapshot['selected'] and normalized not in seen:
                    seen.add(normalized);append.append(raw.rstrip(b'\n')+b'\n')
    fd=os.open(target,os.O_WRONLY|os.O_APPEND|os.O_CREAT,0o600)
    try:
        fcntl.flock(fd,fcntl.LOCK_EX)
        for raw in append:
            view=memoryview(raw)
            while view: view=view[os.write(fd,view):]
        os.fsync(fd)
    finally: os.close(fd)

def handoff(snapshot: dict) -> tuple[set[str],set[Path]]:
    if not snapshot['legacy']: raise TransitionError('legacy transition is already complete')
    active, databases=writers(snapshot)
    shared=snapshot['stores'][0]
    for root in snapshot['stores']:
        for name in DIRECTORIES+FILES:
            path=root/name
            stage=path.with_name(f'.visibility-{os.getpid()}-{name}')
            if stage.exists() or stage.is_symlink(): raise TransitionError('transition staging collision')
            if path.is_symlink() and (root==shared or path.readlink()!=shared/name): raise TransitionError('unexpected shared-state alias')
            if path.exists() and not path.is_symlink() and ((name in DIRECTORIES and not path.is_dir()) or (name in FILES and not path.is_file())): raise TransitionError('unexpected shared-state entry type')
    private(shared)
    with contextlib.ExitStack() as stack:
        locks=[]
        for root in snapshot['stores']:
            private(root)
            directory=root/'thread-writer-locks';private(directory)
            lock=stack.enter_context((directory/'.coordination.lock').open('a+b'))
            fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);locks.append(lock)
        active,databases=writers(snapshot)
        for name in DIRECTORIES: private(shared/name)
        lock_moves=[]
        # Validate every lock before the first payload mutation.
        for root in snapshot['stores'][1:]:
            for lock in (root/'thread-writer-locks').iterdir():
                if lock.name=='.coordination.lock': continue
                if lock.is_symlink() or not lock.is_file(): raise TransitionError('unsafe writer lock')
                target=shared/'thread-writer-locks'/lock.name
                if target.exists():
                    if target.stat().st_ino!=lock.stat().st_ino:
                        busy=[]
                        for file in (target,lock):
                            try:
                                with file.open('a+b') as probe: fcntl.flock(probe,fcntl.LOCK_EX|fcntl.LOCK_NB)
                                busy.append(False)
                            except BlockingIOError: busy.append(True)
                        if all(busy): raise TransitionError('two active writer locks share a thread identity')
                        if busy[1]: lock_moves.append((lock,target))
                else: lock_moves.append((lock,target))
        for source,target in lock_moves:
            try: os.replace(source,target)
            except FileNotFoundError: pass  # Its owner released and removed the lock.
        for identity,entry in snapshot['selected'].items():
            canonical=[c for c in snapshot['records'][identity] if c['root']==shared]
            relative=entry['relative']
            target=shared/relative
            parent=target.parent
            missing=[]
            while not parent.exists(): missing.append(parent);parent=parent.parent
            if parent.is_symlink(): raise TransitionError('substituted canonical session parent')
            for directory in reversed(missing): directory.mkdir(mode=0o700)
            if target.exists() and not target.is_symlink() and target.stat().st_ino==entry['inode'][1]: continue
            os.replace(entry['path'],target)
            for copy in canonical:
                if copy['path']!=target: copy['path'].unlink()
        # Historical leaf aliases are redundant after every selected real payload
        # has been published. Remove them before profile directories become aliases
        # themselves, which would otherwise create broken or circular paths.
        for directory in ('sessions', 'archived_sessions'):
            for path in (shared/directory).rglob('*.jsonl'):
                if path.is_symlink(): path.unlink()
        for name in ('history.jsonl','session_index.jsonl'): append_shared_records(snapshot,name)
        for root in snapshot['stores'][1:]:
            for name in DIRECTORIES+FILES: alias(root/name,shared/name)
            if root not in databases:
                for pattern in ('state_*.sqlite*','thread_history_*.sqlite*'):
                    for path in root.glob(pattern):
                        if path.is_symlink() or not path.is_file(): raise TransitionError('unsafe inactive projection')
                        path.unlink()
    return active,databases

def summary(snapshot: dict) -> dict:
    return {'retained':len(snapshot['selected']),'excluded':len(snapshot['records'])-len(snapshot['selected']),'profiles':len(snapshot['stores'])-1,'legacy_transition_required':snapshot['legacy']}

def main() -> None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation',choices=('plan','apply'))
    parser.add_argument('--home',type=Path,required=True)
    parser.add_argument('--core',type=Path)
    args=parser.parse_args()
    snapshot=inspect(args.home,time.time()-7*86400)
    result=summary(snapshot)
    if args.operation=='apply':
        if args.core is None or not args.core.is_absolute(): raise TransitionError('apply requires the qualified absolute Core entrypoint')
        active,databases=handoff(snapshot)
        # Upstream alone owns canonical SQLite repair, deletion and history projection.
        from shared_state_migrate import _AppServerClient, APP_SERVER_SOURCES
        client=_AppServerClient(str(args.core),snapshot['stores'][0],runtime_home=args.home)
        try:
            client.call('initialize',{'clientInfo':{'name':'codex-visibility-transition','version':'1'},'capabilities':{'experimentalApi':True}});client.notify('initialized')
            for identity,copies in snapshot['records'].items():
                if identity not in snapshot['selected'] and any(c['root']==snapshot['stores'][0] for c in copies):
                    client.call('thread/delete',{'threadId':identity})
            for archived in (False,True):
                client.call('thread/list',{'limit':MAX_ENTRIES,'sourceKinds':APP_SERVER_SOURCES,'archived':archived,'useStateDbOnly':False})
            resumed=0
            for identity,entry in snapshot['selected'].items():
                if identity in active: continue
                client.call('thread/resume',{'threadId':identity,'excludeTurns':True})
                client.call('thread/turns/list',{'threadId':identity,'limit':100})
                names=[snapshot['metadata'][c['root']].get(identity,{}).get('name') for c in snapshot['records'][identity]]
                name=next((n for n in reversed(names) if n),None)
                if name: client.call('thread/name/set',{'threadId':identity,'name':name})
                client.call('thread/unsubscribe',{'threadId':identity});resumed+=1
            result.update(resumed=resumed,active_retained=len(active),pretransition_database_homes=len(databases))
        finally: client.close()
    print(json.dumps(result,sort_keys=True))

if __name__=='__main__':
    try: main()
    except Exception as error:
        # Never expose provider errors, SQLite values or transcript bytes.
        raise SystemExit('shared visibility transition stopped: '+(str(error) if isinstance(error,TransitionError) or type(error).__name__=='MigrationError' else type(error).__name__))
