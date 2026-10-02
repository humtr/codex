from __future__ import annotations
import fcntl
import datetime as dt
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import time
import unittest
import uuid
from scripts import shared_visibility_transition as transition

class VisibilityTransitionTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='visibility-')
        self.home=Path(self.temp.name)
        self.shared=self.home/'.codex';self.shared.mkdir(mode=0o700)
        (self.home/'.codex-profiles').mkdir(mode=0o700)
        self.a=self.home/'.codex-profiles/a';self.a.mkdir(mode=0o700)
        self.b=self.home/'.codex-profiles/b';self.b.mkdir(mode=0o700)
        self.recent=str(uuid.uuid4());self.other=str(uuid.uuid4());self.old=str(uuid.uuid4())
        self.short=self.write(self.shared,self.recent,0,0)
        self.long=self.write(self.a,self.recent,0,2)
        self.write(self.b,self.other,0,1)
        self.write(self.b,self.old,20*86400,1)
        for root,identity in [(self.a,self.recent),(self.b,self.other)]:
            (root/'auth.json').write_bytes(b'fixture-auth-'+root.name.encode())
            (root/'config.toml').write_bytes(b'fixture-config-'+root.name.encode())
            con=sqlite3.connect(root/'state_5.sqlite');con.execute('create table threads(id text, updated_at_ms integer, name text)');con.execute('insert into threads values(?,?,?)',(identity,int(time.time()*1000),'fixture-name'));con.commit();con.close()
            (root/'history.jsonl').write_text(json.dumps({'session_id':identity,'ts':1,'text':'fixture-history'})+'\n')
            (root/'session_index.jsonl').write_text(json.dumps({'id':identity,'thread_name':'fixture-name','updated_at':'2026-10-02T00:00:00Z'})+'\n')
    def tearDown(self): self.temp.cleanup()
    def write(self,root,identity,age,extra):
        folder=root/'sessions';folder.mkdir(mode=0o700,exist_ok=True)
        path=folder/(identity+'.jsonl')
        timestamp=dt.datetime.fromtimestamp(time.time()-age,dt.timezone.utc).isoformat()
        # Identical metadata for the copies of one UUID; creator metadata survives.
        if identity==self.recent: timestamp='2026-10-02T00:00:00Z'
        lines=[{'timestamp':timestamp,'type':'session_meta','payload':{'id':identity,'cwd':'/fixture/cwd' if identity==self.recent else '/fixture/other','creator_account_id':'fixture-account-a' if identity==self.recent else 'fixture-account-b'}}]
        lines.extend({'timestamp':timestamp,'type':'response_item','payload':{'type':'message','text':str(i)}} for i in range(extra))
        path.write_text(''.join(json.dumps(row)+'\n' for row in lines));return path
    def snapshot(self): return transition.inspect(self.home,time.time()-7*86400)
    def proc(self,paths):
        root=self.home/'proc';root.mkdir(exist_ok=True)
        for index,path in enumerate(paths,1):
            p=root/str(index);p.mkdir(exist_ok=True);(p/'fd').mkdir(exist_ok=True);(p/'exe').symlink_to('/qualified/runtime');(p/'fd/1').symlink_to(path)
        return root
    def test_recent_union_preserves_history_auth_and_active_inode_through_atomic_exchange(self):
        (self.shared/'sessions/alias.jsonl').symlink_to(self.long)
        (self.shared/'sessions/expired-alias.jsonl').symlink_to(self.b/'sessions/missing.jsonl')
        snapshot=self.snapshot()
        self.assertEqual(set(snapshot['selected']),{self.recent,self.other})
        self.assertEqual(snapshot['selected'][self.recent]['path'],self.long)
        before={root:((root/'auth.json').read_bytes(),(root/'config.toml').read_bytes()) for root in (self.a,self.b)}
        inode=self.long.stat().st_ino
        with self.long.open('ab',buffering=0) as writer:
            transition.handoff(snapshot)
            writer.write(b'{"type":"event_msg","payload":{"type":"fixture"}}\n')
        self.assertEqual(self.short.stat().st_ino,inode)
        self.assertIn(b'fixture',self.short.read_bytes())
        for root in (self.a,self.b):
            for name in transition.DIRECTORIES+transition.FILES: self.assertEqual((root/name).readlink(),self.shared/name)
            self.assertEqual(((root/'auth.json').read_bytes(),(root/'config.toml').read_bytes()),before[root])
            self.assertFalse((root/'state_5.sqlite').exists())
        self.assertEqual(len(list((self.shared/'sessions').glob('*.jsonl'))),2)
        self.assertEqual(len((self.shared/'history.jsonl').read_text().splitlines()),2)
        self.assertFalse((self.b/'sessions'/self.old).exists())
        self.assertFalse(any(self.home.rglob('.visibility-*')))
        self.assertFalse(any('backup' in p.name for p in self.home.rglob('*')))
        self.assertFalse(self.snapshot()['legacy'])
    def test_active_excluded_or_nonmaximal_writer_refuses_handoff(self):
        snapshot=self.snapshot()
        proc=self.proc([self.short])
        with self.assertRaises(transition.TransitionError): transition.writers(snapshot,proc)
        (proc/'1/fd/1').unlink();(proc/'1/fd/1').symlink_to(self.long)
        active,_=transition.writers(snapshot,proc);self.assertEqual(active,{self.recent})
        (proc/'1/fd/1').unlink();(proc/'1/fd/1').symlink_to(self.b/'sessions'/(self.old+'.jsonl'))
        with self.assertRaises(transition.TransitionError): transition.writers(snapshot,proc)
        self.assertFalse((self.a/'sessions').is_symlink())
    def test_divergence_dependency_and_escaping_alias_are_readonly_failures(self):
        original=self.short.read_bytes()
        with self.short.open('ab') as f:f.write(b'{"type":"response_item","payload":{"text":"conflict"}}\n')
        with self.assertRaises(transition.TransitionError):self.snapshot()
        self.short.write_bytes(original)
        con=sqlite3.connect(self.a/'state_5.sqlite');con.execute('create table thread_dynamic_tools(thread_id text)');con.execute('insert into thread_dynamic_tools values(?)',(self.recent,));con.commit();con.close()
        with self.assertRaises(transition.TransitionError):self.snapshot()
        con=sqlite3.connect(self.a/'state_5.sqlite');con.execute('delete from thread_dynamic_tools');con.commit();con.close()
        (self.a/'sessions/escape.jsonl').symlink_to(self.a/'auth.json')
        with self.assertRaises(transition.TransitionError):self.snapshot()
        self.assertEqual(self.short.read_bytes(),original)
        self.assertFalse((self.a/'sessions').is_symlink())
    def test_active_lock_inode_moves_and_two_active_copies_refuse_before_payload_move(self):
        a=self.a/'thread-writer-locks';a.mkdir(mode=0o700)
        c=self.shared/'thread-writer-locks';c.mkdir(mode=0o700)
        source=a/(self.recent+'.lock');target=c/(self.recent+'.lock')
        source.write_bytes(b'');target.write_bytes(b'')
        with source.open('a+b') as one, target.open('a+b') as two:
            fcntl.flock(one,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(two,fcntl.LOCK_EX|fcntl.LOCK_NB)
            with self.assertRaises(transition.TransitionError):transition.handoff(self.snapshot())
            self.assertTrue(self.long.exists())
            self.assertFalse((self.a/'sessions').is_symlink())
            fcntl.flock(two,fcntl.LOCK_UN)
            inode=source.stat().st_ino
            transition.handoff(self.snapshot())
            self.assertEqual(target.stat().st_ino,inode)
            with target.open('a+b') as probe:
                with self.assertRaises(BlockingIOError):fcntl.flock(probe,fcntl.LOCK_EX|fcntl.LOCK_NB)

    def test_exchange_failure_does_not_remove_original_and_alias_rejects_staging_collision(self):
        target=self.home/'target';target.mkdir(mode=0o700)
        stage=self.a/f'.visibility-{os.getpid()}-sessions'
        stage.write_bytes(b'collision')
        with self.assertRaises(transition.TransitionError):transition.alias(self.a/'sessions',target)
        self.assertTrue(self.long.exists())
        with self.assertRaises(OSError):transition.exchange(self.a/'sessions',self.home/'absent')
        self.assertTrue(self.long.exists())

if __name__=='__main__':unittest.main()
