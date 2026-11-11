"""Exercise the actual frontend assembler with disposable upstream trees."""
from pathlib import Path
import subprocess
import json
import shutil
import os
import tempfile
import unittest
from unittest import mock

from apply import apply, check_scope


class ReentryEnvironmentTests(unittest.TestCase):
    def test_reentry_marker_is_consumed_once_before_child_execution(self):
        package = Path(__file__).resolve().parent
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'capture.rs'
            source.write_text(r'''#[path = @MODULE@]
mod termux_reentry;
fn main() {
    use std::os::unix::process::CommandExt;
    let args: Vec<String> = std::env::args().collect();
    let expected = match args[2].as_str() {
        "select" => Some(termux_reentry::Intent::Select),
        "resume" => Some(termux_reentry::Intent::Resume),
        _ => None,
    };
    if args[1] == "bind" || args[1] == "unknown" {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.args(["consume", &args[2], &args[3]]);
        if args[1] == "bind" {
            termux_reentry::configure_exec(&mut command, expected);
        } else {
            command.env(termux_reentry::ENV, "unknown")
                .env(termux_reentry::OWNER_PID_ENV, std::process::id().to_string());
        }
        panic!("exec failed: {}", command.exec());
    }
    // This fixture is a real synchronous main, not a multithreaded test body.
    let actual = unsafe { termux_reentry::take_before_threads() };
    assert_eq!(actual, expected);
    assert!(std::env::var_os(termux_reentry::ENV).is_none());
    assert!(std::env::var_os(termux_reentry::OWNER_PID_ENV).is_none());
    assert_eq!(unsafe { termux_reentry::take_before_threads() }, None);
    let output = std::process::Command::new(&args[3])
        .args(["-c", "if printenv CODEX_CHAT_MODEL_REENTRY >/dev/null || printenv CODEX_CHAT_MODEL_REENTRY_PID >/dev/null; then exit 9; fi; printf REENTRY_CHILD_CLEAN"])
        .output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"REENTRY_CHILD_CLEAN");
    assert!(output.stderr.is_empty());
    println!("REENTRY_CONSUMED_ONCE_AND_CHILD_CLEAN");
}
'''.replace('@MODULE@', json.dumps(str(package / 'src/termux_reentry.rs'))))
            binary = root / 'capture'
            subprocess.run([shutil.which('rustc') or 'rustc', '--edition=2024', '-Dwarnings',
                            str(source), '-o', str(binary)], check=True)
            shell = shutil.which('sh')
            self.assertIsNotNone(shell)
            cases = [
                ('bind', 'select', {}),
                ('bind', 'resume', {}),
                ('consume', 'none', {}),
                ('consume', 'none', {'CODEX_CHAT_MODEL_REENTRY': 'resume'}),
                ('consume', 'none', {'CODEX_CHAT_MODEL_REENTRY': 'resume',
                                     'CODEX_CHAT_MODEL_REENTRY_PID': str(os.getpid())}),
                ('consume', 'none', {'CODEX_CHAT_MODEL_REENTRY': 'resume',
                                     'CODEX_CHAT_MODEL_REENTRY_PID': 'invalid'}),
                ('unknown', 'none', {}),
            ]
            for stage, expected, supplied in cases:
                with self.subTest(stage=stage, expected=expected, supplied=supplied):
                    environment = dict(os.environ)
                    environment.pop('CODEX_CHAT_MODEL_REENTRY', None)
                    environment.pop('CODEX_CHAT_MODEL_REENTRY_PID', None)
                    environment.update(supplied)
                    result = subprocess.run([str(binary), stage, expected, shell],
                                            env=environment, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(result.stdout, 'REENTRY_CONSUMED_ONCE_AND_CHILD_CLEAN\n')

    def test_both_actual_entrypoints_capture_before_arg0_runtime(self):
        patch = (Path(__file__).resolve().parent / 'native.patch').read_text()
        for path in ['codex-rs/cli/src/main.rs', 'codex-rs/tui/src/main.rs']:
            with self.subTest(path=path):
                section = patch.split('diff --git a/' + path + ' b/' + path + '\n', 1)[1]
                section = section.split('\ndiff --git ', 1)[0]
                result = '\n'.join(line[1:] for line in section.splitlines()
                                   if line.startswith((' ', '+')) and not line.startswith('+++'))
                self.assertEqual(result.count('termux_reentry::take_before_threads()'), 1)
                self.assertLess(result.index('termux_reentry::take_before_threads()'),
                                result.index('arg0_dispatch_or_else('))


class AssemblyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[2]
        target = Path(os.environ.get('CARGO_TARGET_DIR', Path(os.environ.get('XDG_CACHE_HOME', Path.home()/'.cache'))/'codex/check-target')).resolve()
        environment = {**os.environ, 'CARGO_TARGET_DIR':str(target), 'CARGO_PROFILE_DEV_DEBUG':'0', 'CARGO_INCREMENTAL':'0'}
        subprocess.run(['cargo','build','--locked','-p','codex-release-builder','--manifest-path',str(root/'Cargo.toml')],
                       env=environment,check=True,stdout=subprocess.DEVNULL)
        cls.builder = target/'debug/codex-release-builder'

    def test_rebase_retains_prepared_native_changes_and_real_index(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            path = target / 'lib.rs'
            original = ''.join(f'line {i}\n' for i in range(12))
            path.write_text(original)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(upstream), *args])
            git('add', '.')
            git('-c', 'user.name=Owned', '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'official')
            index = git('write-tree')
            path.write_text(original.replace('line 6', 'integration'))
            (package / 'native.patch').write_bytes(b'# codex-upstream-base '+git('rev-parse','HEAD').strip()+b'\n'+git('diff', '--binary'))
            path.write_text(original.replace('line 3', 'prepared Native'))
            self.assertEqual(apply(upstream, package, self.builder), 1)
            self.assertEqual(path.read_text(), original.replace('line 3', 'prepared Native').replace('line 6', 'integration'))
            self.assertEqual(git('write-tree'), index)

    def test_rebase_locates_changed_context_and_conflict_leaves_tree_and_index_unchanged(self):
        for conflict in [False, True]:
            with self.subTest(conflict=conflict), tempfile.TemporaryDirectory() as temporary:
                upstream, package, target = self.fixture(Path(temporary))
                path = target / 'lib.rs'
                original = ''.join(f'line {i}\n' for i in range(12))
                path.write_text(original)
                def git(*args):
                    return subprocess.check_output(['git', '-C', str(upstream), *args])
                git('config', 'user.name', 'Owned'); git('config', 'user.email', 'owned@example.invalid')
                git('add', '.'); git('commit', '-qm', 'official base')
                path.write_text(original.replace('line 6', 'integration'))
                (package / 'native.patch').write_bytes(b'# codex-upstream-base '+git('rev-parse','HEAD').strip()+b'\n'+git('diff', '--binary'))
                shifted = original.replace('line 6' if conflict else 'line 3', 'upstream change')
                path.write_text(shifted)
                git('add', '.'); git('commit', '-qm', 'official next release')
                before_index = git('write-tree')
                if conflict:
                    with self.assertRaises(subprocess.CalledProcessError):
                        apply(upstream, package, self.builder)
                    self.assertEqual(path.read_text(), shifted)
                    self.assertFalse((target / 'app/owned.rs').exists())
                else:
                    self.assertEqual(apply(upstream, package, self.builder), 1)
                    self.assertEqual(path.read_text(), shifted.replace('line 6', 'integration'))
                self.assertEqual(git('write-tree'), before_index)

    def test_merge_follows_an_upstream_file_rename(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            def git(*args):return subprocess.check_output(['git','-C',str(upstream),*args])
            path=target/'lib.rs';original=''.join(f'line {i}\n' for i in range(12))
            path.write_text(original);git('add','.');git('-c','user.name=Owned','-c','user.email=owned@localhost','commit','-qm','base')
            base=git('rev-parse','HEAD').strip()
            path.write_text(original.replace('line 6','integration'))
            (package/'native.patch').write_bytes(b'# codex-upstream-base '+base+b'\n'+git('diff','--binary'))
            path.write_text(original);git('mv','codex-rs/tui/src/lib.rs','codex-rs/tui/src/relocated.rs')
            git('-c','user.name=Owned','-c','user.email=owned@localhost','commit','-qam','upstream move')
            index=git('write-tree')
            apply(upstream,package,self.builder)
            self.assertFalse(path.exists())
            self.assertEqual((target/'relocated.rs').read_text(),original.replace('line 6','integration'))
            self.assertEqual(git('write-tree'),index)

    def test_scope_accepts_catalog_config_and_tui_but_rejects_other_source_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            core = upstream / 'codex-rs/core/src/lib.rs'
            core.parent.mkdir(parents=True)
            core.write_text('original\n')
            outside = upstream / 'codex-rs/outside.rs'
            outside.write_text('original\n')
            subprocess.run(['git', '-C', str(upstream), 'add', '.'], check=True)
            subprocess.run(['git', '-C', str(upstream), '-c', 'user.name=Owned',
                            '-c', 'user.email=owned@example.invalid', 'commit', '-qm', 'owned'], check=True)
            apply(upstream, package, self.builder)
            config = core.parent / 'config/mod.rs'
            config.parent.mkdir()
            config.write_text('complete catalog source\n')
            (upstream / 'codex-rs/Cargo.lock').write_text('normalized release workspace\n')
            native_files = {b'codex-rs/core/src/config/mod.rs'}
            schema = upstream / 'codex-rs/app-server-protocol/schema/precomputed'
            schema.mkdir(parents=True)
            (schema / 'app-server-exports-stable.json.zst').write_bytes(b'upstream generated protocol output')
            entry = upstream / 'codex-rs/cli/src/main.rs'
            entry.parent.mkdir(parents=True)
            entry.write_text('consume private startup intent before threads\n')
            check_scope(upstream, native_files)
            for path in [core, outside, core.parent / 'new.rs', upstream / 'new.rs',
                         upstream / 'codex-rs/cli/src/lib.rs',
                         upstream / 'codex-rs/cli/Cargo.toml',
                         upstream / 'codex-rs/app-server-protocol/unrelated.rs']:
                with self.subTest(path=path.relative_to(upstream)):
                    before = path.read_bytes() if path.exists() else None
                    path.write_text('unrelated change\n')
                    with self.assertRaises(ValueError):
                        check_scope(upstream, native_files)
                    if before is None:
                        path.unlink()
                    else:
                        path.write_bytes(before)
            check_scope(upstream, native_files)
            # Moving an unrelated source into TUI must still expose its deletion.
            outside.rename(target / 'moved.rs')
            subprocess.run(['git', '-C', str(upstream), 'add', '-A'], check=True)
            with self.assertRaises(ValueError):
                check_scope(upstream, native_files)

    def fixture(self, root):
        upstream, package = root / 'upstream', root / 'package'
        target = upstream / 'codex-rs/tui/src'
        target.mkdir(parents=True)
        (target / 'lib.rs').write_text('upstream\n')
        (package / 'src/app').mkdir(parents=True)
        (package / 'src/app/owned.rs').write_text('owned implementation\n')
        (package / 'native.patch').write_text(
            'diff --git a/codex-rs/tui/src/lib.rs b/codex-rs/tui/src/lib.rs\n'
            '--- a/codex-rs/tui/src/lib.rs\n+++ b/codex-rs/tui/src/lib.rs\n'
            '@@ -1 +1,2 @@\n upstream\n+integration\n')
        subprocess.run(['git', 'init', '-q', str(upstream)], check=True)
        subprocess.run(['git','-C',str(upstream),'add','.'],check=True)
        subprocess.run(['git','-C',str(upstream),'-c','user.name=Owned fixture','-c','user.email=owned@localhost','commit','-qm','official'],check=True)
        base=subprocess.check_output(['git','-C',str(upstream),'rev-parse','HEAD']).strip()
        patch=package/'native.patch'
        patch.write_bytes(b'# codex-upstream-base '+base+b'\n'+patch.read_bytes())
        return upstream, package, target

    def test_real_assembly_preserves_sources_and_refuses_second_application(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            self.assertEqual(apply(upstream, package, self.builder), 1)
            self.assertEqual((target / 'lib.rs').read_text(), 'upstream\nintegration\n')
            self.assertEqual((target / 'app/owned.rs').read_bytes(),
                             (package / 'src/app/owned.rs').read_bytes())
            with self.assertRaises(ValueError):
                apply(upstream, package, self.builder)

    def test_patch_owned_collision_leaves_sources_and_real_index_unchanged(self):
        with tempfile.TemporaryDirectory() as temporary:
            upstream, package, target = self.fixture(Path(temporary))
            patch = package / 'native.patch'
            patch.write_text(patch.read_text() +
                'diff --git a/codex-rs/tui/src/app/owned.rs b/codex-rs/tui/src/app/owned.rs\n'
                'new file mode 100644\n--- /dev/null\n'
                '+++ b/codex-rs/tui/src/app/owned.rs\n'
                '@@ -0,0 +1 @@\n+conflicting patch implementation\n')
            before_index = subprocess.check_output(['git', '-C', str(upstream), 'write-tree'])
            with self.assertRaises(subprocess.CalledProcessError):
                apply(upstream, package, self.builder)
            self.assertEqual((target / 'lib.rs').read_text(), 'upstream\n')
            self.assertFalse((target / 'app/owned.rs').exists())
            self.assertEqual(
                subprocess.check_output(['git', '-C', str(upstream), 'write-tree']),
                before_index)

    def test_owned_input_disappearing_after_preflight_leaves_checkout_unchanged(self):
        for fault in ['missing-root', 'symlink-payload']:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                upstream, package, target = self.fixture(root)
                sentinel = root / 'outside.rs'
                sentinel.write_text('unrelated source\n')
                before_index = subprocess.check_output(['git', '-C', str(upstream), 'write-tree'])
                run = subprocess.run

                def remove_preflight_input(*args, **kwargs):
                    if fault == 'missing-root':
                        (package / 'src').rename(package / 'saved-src')
                    else:
                        payload = package / 'src/app/owned.rs'
                        payload.unlink()
                        payload.symlink_to(sentinel)
                    return run(*args, **kwargs)

                with mock.patch('apply.subprocess.run', side_effect=remove_preflight_input):
                    with self.assertRaises(subprocess.CalledProcessError):
                        apply(upstream, package, self.builder)
                self.assertEqual((target / 'lib.rs').read_text(), 'upstream\n')
                self.assertFalse((target / 'app/owned.rs').exists())
                self.assertEqual(sentinel.read_text(), 'unrelated source\n')
                self.assertEqual(
                    subprocess.check_output(['git', '-C', str(upstream), 'write-tree']),
                    before_index)

    def test_collisions_substitutions_and_bad_patch_fail_before_mutation(self):
        for fault in ('collision', 'source-link', 'parent-link', 'bad-patch'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                upstream, package, target = self.fixture(root)
                sentinel = root / 'outside'; sentinel.mkdir()
                if fault == 'collision':
                    (target / 'app').mkdir()
                    (target / 'app/owned.rs').write_text('real upstream file\n')
                elif fault == 'source-link':
                    (package / 'src/linked.rs').symlink_to(target / 'lib.rs')
                elif fault == 'parent-link':
                    (target / 'app').symlink_to(sentinel, target_is_directory=True)
                else:
                    (package / 'native.patch').write_text('invalid patch\n')
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    apply(upstream, package, self.builder)
                self.assertEqual((target / 'lib.rs').read_text(), 'upstream\n')
                self.assertEqual(list(sentinel.iterdir()), [])
                if fault == 'collision':
                    self.assertEqual((target / 'app/owned.rs').read_text(), 'real upstream file\n')


if __name__ == '__main__':
    unittest.main()
