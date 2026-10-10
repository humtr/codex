#!/usr/bin/env python3
"""Real private Manager entrypoint and temporary tmux/process identity proof."""
import argparse
import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("manager", type=Path)
    options = parser.parse_args()
    manager = options.manager.resolve()
    with tempfile.TemporaryDirectory(prefix="codex-terminal-proof-") as temporary:
        root = Path(temporary)
        native = root / ".local/lib/codex/profile-tui-preview/direct-selection-v1/native"
        native.parent.mkdir(parents=True)
        source = root / "caller.c"
        source.write_text(r'''#include <unistd.h>
#include <sys/wait.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <fcntl.h>
#include <sys/stat.h>
int main(int argc, char **argv) {
  if (argc != 4) return 2;
  char path[4096]; snprintf(path, sizeof path, "%s/.codex-terminal-thread-v1-%d-XXXXXX", getenv("TMPDIR"), getpid());
  int fd = mkstemp(path);
  unlink(path);
  fcntl(fd, F_SETFD, FD_CLOEXEC);
  if (fd < 0 || fchmod(fd, 0600)) return 4;
  char descriptor[32]; snprintf(descriptor, sizeof descriptor, "%d", fd);
  for (;;) {
    FILE *request = fopen(argv[2], "r");
    if (request) {
      char id[128] = {0};
      if (!fgets(id, sizeof id, request)) return 3;
      fclose(request); unlink(argv[2]);
      ftruncate(fd, 0);
      if (strcmp(id, "none")) pwrite(fd, id, strlen(id), 0);
      int child = fork();
      if (child == 0) { execl(argv[1], argv[1], "__terminal-bind-v1", descriptor, NULL); _exit(127); }
      int status = 0; waitpid(child, &status, 0);
      FILE *reply = fopen(argv[3], "w");
      fprintf(reply, "%d", WIFEXITED(status) ? WEXITSTATUS(status) : 128); fclose(reply);
    }
    usleep(10000);
  }
}
''')
        subprocess.run(["cc", str(source), "-o", str(native)], check=True, capture_output=True)
        socket = str(root / "tmux.sock")
        request = root / "request"
        reply = root / "reply"
        env = dict(os.environ, HOME=str(root), CODEX_TERMUX_CORE_API="codex-manager-core-v1",
                   CODEX_TERMUX_CORE_ENTRYPOINT=str(native), TMUX="", TMPDIR=str(root))
        def tm(*args):
            return subprocess.check_output(["tmux", "-S", socket, *args], env=env, text=True).strip()
        import shlex
        import sys
        wrapper = root / '.config/ai/lib/ai_run.py'
        wrapper.parent.mkdir(parents=True)
        wrapper.write_text("import json,os,pty,signal,subprocess,sys,time\nplan=json.loads(sys.argv[1])\nmaster,slave=pty.openpty()\nchild=subprocess.Popen(plan['fixture_argv'],stdin=slave,stdout=slave,stderr=slave)\ndef stop(*unused):\n child.terminate();child.wait(timeout=2);sys.exit(0)\nsignal.signal(signal.SIGHUP,stop)\nwhile child.poll() is None: time.sleep(.02)\n")
        first = "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3"
        second = "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b4"
        for mode in ['direct', 'supervised', 'wrong-plan', 'wrong-script']:
            caller = [str(native), str(manager), str(request), str(reply)]
            if mode != 'direct':
                script = wrapper
                if mode == 'wrong-script':
                    script = root / 'untrusted-wrapper.py'
                    shutil.copyfile(wrapper, script)
                plan = {'auth_recovery': mode != 'wrong-plan', 'argv':['codex'], 'fixture_argv':caller}
                caller = [sys.executable, str(script), json.dumps(plan)]
            request.unlink(missing_ok=True)
            reply.unlink(missing_ok=True)
            pane = tm('-f','/dev/null','new-session','-d','-P','-F','#{pane_id}','-s','proof',shlex.join(caller))
            try:
                def publish(id):
                    reply.unlink(missing_ok=True)
                    request.write_text(id)
                    deadline = time.monotonic()+3
                    while not reply.exists() and time.monotonic()<deadline: time.sleep(.01)
                    assert reply.exists(), 'private endpoint did not complete'
                    return int(reply.read_text())
                if mode.startswith('wrong'):
                    assert publish(first) == 1
                    assert tm('show-options','-pqv','-t',pane,'@codex_terminal_binding_v1') == ''
                    print('PASS private entrypoint refused '+mode)
                    continue
                assert publish(first) == 0
                binding = tm('show-options','-pqv','-t',pane,'@codex_terminal_binding_v1')
                parts = binding.split(':')
                rootpid = tm('display-message','-p','-t',pane,'#{pane_pid}')
                assert len(parts) == 5 and parts[2] == rootpid
                nativepid = parts[0]
                if mode == 'direct': assert nativepid == rootpid
                else: assert nativepid != rootpid
                for pid, start in [(nativepid,parts[1]),(rootpid,parts[3])]:
                    assert Path('/proc',pid,'stat').read_text().rpartition(')')[2].split()[19] == start
                slot = Path('/proc',nativepid,'fd',parts[4])
                assert slot.read_text() == first
                assert publish(second) == 0
                assert tm('show-options','-pqv','-t',pane,'@codex_terminal_binding_v1') == binding
                assert slot.read_text() == second
                assert publish('none') == 0 and slot.read_bytes() == b''
                assert publish('$(false)') == 1
                assert tm('show-options','-pqv','-t',pane,'@codex_terminal_binding_v1') == binding
                foreign = subprocess.run([str(manager),'__terminal-bind-v1',parts[4]],env=dict(env,TMUX=socket+',1,0',TMUX_PANE=pane),capture_output=True)
                assert foreign.returncode == 1
                assert subprocess.run([str(manager),'__terminal-bind-v1','invalid'],env=env,capture_output=True).returncode == 2
                print('PASS private entrypoint '+mode+': full ID, switch, empty, invalid payload/grammar, foreign caller')
            finally:
                subprocess.run(['tmux','-S',socket,'kill-server'],capture_output=True)


if __name__ == "__main__":
    main()
