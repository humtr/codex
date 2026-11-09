"""Real builder/publication -> local fixture boundary; no Android execution."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

SOURCE = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SOURCE / 'scripts'))
import qualify_permission_lifecycle as permission
from qualify_tasks import local_generation_inventory, materialize_signed_generation
from release_preflight import PreflightError, parse_release_manifest_inventory
from release_publication import ValidationError, parse_manifest


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def elf(android=False, runtime=False):
    interpreter = b'/system/bin/linker64\0' if android else b''
    data = bytearray(120 + len(interpreter))
    data[:7] = b'\x7fELF\x02\x01\x01'
    for offset, form, value in ((16, 'H', 3 if android else 2), (18, 'H', 183),
                               (20, 'I', 1), (32, 'Q', 64), (52, 'H', 64),
                               (54, 'H', 56), (56, 'H', 1), (64, 'I', 3 if android else 1)):
        struct.pack_into('<' + form, data, offset, value)
    if android:
        for offset, value in ((72, 120), (96, len(interpreter)), (104, len(interpreter))):
            struct.pack_into('<Q', data, offset, value)
        data[120:] = interpreter
    if runtime:
        data += b'\0'.join([b'/etc/resolv.conf', b'/etc/codex/managed_config.toml',
                            b'/etc/codex/config.toml', b'/etc/codex/requirements.toml',
                            b'/etc/resolv.conf']) + b'\0'
    return bytes(data)


class LocalFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        target = Path(os.environ.get('CARGO_TARGET_DIR',
                      Path(os.environ.get('XDG_CACHE_HOME', Path.home() / '.cache')) / 'codex/check-target')).resolve()
        env = {**os.environ, 'CARGO_TARGET_DIR': str(target),
               'CARGO_PROFILE_DEV_DEBUG': '0', 'CARGO_INCREMENTAL': '0'}
        subprocess.run(['cargo', 'build', '--locked', '-p', 'codex-release-builder',
                        '--manifest-path', str(SOURCE / 'Cargo.toml')], env=env, check=True,
                       stdout=subprocess.DEVNULL)
        cls.builder = target / 'debug/codex-release-builder'
        cls.temporary = tempfile.TemporaryDirectory(prefix='local-fixture-producer-')
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.root = Path(cls.temporary.name)
        cls.key = cls.root / 'key.pem'
        cls.run_command(['openssl', 'genpkey', '-algorithm', 'Ed25519', '-out', cls.key])
        cls.key.chmod(0o600)
        core, tui, manager = (cls.root / name for name in ('core', 'tui', 'manager'))
        core.write_bytes(elf(android=True)); core.chmod(0o755)
        tui.write_bytes(elf(runtime=True)); tui.chmod(0o755)
        manager.write_text('#!' + shutil.which('sh') + '\n'
                           'test "$1" = --artifact-probe || exit 9\n'
                           "printf '%s\\n' codex-manager-artifact-v1 core_api=codex-manager-core-v1\n")
        manager.chmod(0o755)
        archive = cls.root / 'official-shaped-fixture.tar.gz'
        package = {'layoutVersion': 1, 'version': '0.150.1',
                   'target': 'aarch64-unknown-linux-musl', 'variant': 'codex',
                   'entrypoint': 'bin/codex', 'resourcesDir': 'codex-resources', 'pathDir': 'codex-path'}
        with tarfile.open(archive, 'w:gz', format=tarfile.USTAR_FORMAT) as tar:
            for name, data in {'bin/codex': elf(runtime=True),
                               'bin/codex-code-mode-host': elf(),
                               'codex-package.json': json.dumps(package).encode()}.items():
                info = tarfile.TarInfo(name)
                info.mode, info.size = 0o644, len(data)
                tar.addfile(info, io.BytesIO(data))
        cls.generations = {}
        for label, managed, frontend, bridge in (('full', True, True, False),
                                                 ('base', False, False, False),
                                                 ('tui', False, True, False),
                                                 ('r10-full', True, True, True)):
            identity = 'local-fixture-' + label
            unsigned, published = cls.root / (label + '-unsigned'), cls.root / (label + '-published')
            args = [cls.builder, 'build', '--version', '0.150.1', '--archive', archive,
                    '--archive-sha256', digest(archive), '--generation-id', identity,
                    '--core', core, '--creation-metadata',
                    'r10-browser-helper-bridge-v1' if bridge else 'local-fixture-test',
                    '--gzip', shutil.which('gzip'), '--openssl', shutil.which('openssl'), '--output', unsigned]
            if managed:
                args += ['--manager', manager]
            if frontend:
                args += ['--tui', tui, '--tui-version', '0.150.1']
            if bridge:
                args += ['--legacy-activation-doctor-unsupported']
            cls.run_command(args)
            cls.run_command([cls.builder, 'publish', '--generation', unsigned,
                             '--release-sequence', '10', '--release-base',
                             f'https://humtr.github.io/codex/{identity}/', '--private-key', cls.key,
                             '--openssl', shutil.which('openssl'), '--output', published])
            cls.generations[label] = published / 'releases' / identity

    @classmethod
    def run_command(cls, args):
        result = subprocess.run([str(x) for x in args], capture_output=True, timeout=60)
        if result.returncode:
            raise AssertionError((result.returncode, result.stderr[-4000:]))

    def test_real_canonical_full_producer_enters_shared_permission_fixture(self):
        original = self.generations['full']
        before = {str(p.relative_to(original)): digest(p) for p in original.rglob('*') if p.is_file()}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            prefix = root / 'host-prefix'
            (prefix / 'etc/tls').mkdir(parents=True)
            (prefix / 'etc/resolv.conf').write_text('nameserver 127.0.0.1\n')
            (prefix / 'etc/tls/cert.pem').write_text('owned fixture\n')
            fixture_root = root / 'fixture'
            fixture_root.mkdir(mode=0o700)
            with mock.patch.dict(os.environ, {'PREFIX': str(prefix), 'TMPDIR': str(root)}):
                home, _, _ = permission.fixture(fixture_root, original / 'core', None, generation=original)
            installed = home / '.local/lib/codex/core/generations/local-permission-fixture'
            fields, records = local_generation_inventory(installed)
            self.assertEqual(fields['helper_count'], '3')
            self.assertNotEqual(fields['manager_artifact_digest'], '-')
            self.assertEqual({name for name, _, _ in records},
                             {'core', 'runtime', 'manager', 'codex-code-mode-host', 'generation.meta',
                              'browser/open/curl', 'browser/manual/curl', 'helpers/2'})
            self.assertEqual(digest(installed / 'helpers/2'), digest(original / 'helpers/2'))
            self.run_command(['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin',
                              '-inkey', fixture_root / 'fixture-public.pem',
                              '-in', installed / 'release.manifest', '-sigfile', installed / 'release.sig'])
        self.assertEqual(before, {str(p.relative_to(original)): digest(p)
                                 for p in original.rglob('*') if p.is_file()})

    def test_materializer_preserves_all_builder_layouts_and_public_floor(self):
        for label, original in self.generations.items():
            with self.subTest(layout=label), tempfile.TemporaryDirectory() as temporary:
                destination = Path(temporary) / 'generation'
                materialize_signed_generation(original, destination)
                self.assertEqual(local_generation_inventory(destination), local_generation_inventory(original))
                self.assertEqual(digest(destination / 'release.sig'), digest(original / 'release.sig'))
                self.assertEqual(digest(destination / 'release.manifest'), digest(original / 'release.manifest'))
                if label == 'r10-full':
                    parse_manifest(original / 'release.manifest')
                else:
                    with self.assertRaises(ValidationError):
                        parse_manifest(original / 'release.manifest')

    def test_invalid_inventory_or_path_never_materializes_a_fixture(self):
        cases = ('extra', 'missing', 'digest', 'mode', 'duplicate', 'traversal',
                 'file-symlink', 'parent-symlink', 'fifo', 'root-symlink', 'root-parent-symlink',
                 'wrong-layout', 'wrong-helper', 'wrong-api', 'directory-mode', 'signature-size')
        for case in cases:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = root / 'source'
                shutil.copytree(self.generations['full'], source)
                manifest = source / 'release.manifest'
                if case == 'extra':
                    (source / 'unowned').write_bytes(b'extra')
                elif case == 'missing':
                    (source / 'runtime').unlink()
                elif case == 'digest':
                    (source / 'runtime').write_bytes(b'changed')
                elif case == 'mode':
                    (source / 'runtime').chmod(0o644)
                elif case == 'duplicate':
                    rows = manifest.read_text().splitlines()
                    rows[9] = 'file_count\t9'
                    rows.append(rows[-1])
                    manifest.write_text('\n'.join(rows) + '\n')
                elif case == 'traversal':
                    manifest.write_text(manifest.read_text().replace('file\truntime\t', 'file\t../runtime\t'))
                elif case in ('file-symlink', 'fifo'):
                    outside = root / 'outside'
                    shutil.copy2(source / 'runtime', outside)
                    (source / 'runtime').unlink()
                    if case == 'fifo':
                        os.mkfifo(source / 'runtime', 0o600)
                    else:
                        (source / 'runtime').symlink_to(outside)
                elif case == 'parent-symlink':
                    outside = root / 'outside'
                    (source / 'browser').rename(outside)
                    (source / 'browser').symlink_to(outside, target_is_directory=True)
                elif case == 'root-symlink':
                    alias = root / 'alias'
                    alias.symlink_to(source, target_is_directory=True)
                    source = alias
                elif case == 'root-parent-symlink':
                    store = root / 'store'
                    store.mkdir()
                    source.rename(store / 'source')
                    alias = root / 'alias-parent'
                    alias.symlink_to(store, target_is_directory=True)
                    source = alias / 'source'
                elif case in ('wrong-layout', 'wrong-helper'):
                    descriptor = source / 'generation.meta'
                    text = descriptor.read_text()
                    text = (text.replace('local-fixture-test', 'r10-browser-helper-bridge-v1')
                            if case == 'wrong-layout' else
                            text.replace('termux-manager-tui-v1:0.150.1', 'termux-manager-tui-v1:0.150.2'))
                    descriptor.write_text(text)
                    rows = manifest.read_text().splitlines()
                    rows = ['file\tgeneration.meta\t' + digest(descriptor) + '\t0644'
                            if row.startswith('file\tgeneration.meta\t') else row for row in rows]
                    manifest.write_text('\n'.join(rows) + '\n')
                elif case == 'wrong-api':
                    manifest.write_text(manifest.read_text().replace('core-api-v1', 'core-api-v2'))
                elif case == 'directory-mode':
                    (source / 'browser').chmod(0o777)
                elif case == 'signature-size':
                    (source / 'release.sig').write_bytes(b'not a signature')
                destination = root / 'destination'
                with self.assertRaises((PreflightError, FileNotFoundError)):
                    materialize_signed_generation(source, destination)
                self.assertFalse(destination.exists())

    def test_common_decoder_keeps_ordered_records_without_public_layout_policy(self):
        fields, records = parse_release_manifest_inventory(self.generations['full'] / 'release.manifest')
        self.assertEqual(fields['generation_id'], 'local-fixture-full')
        self.assertEqual(len(records), 8)
        self.assertEqual([name for name, _, _ in records], sorted(name for name, _, _ in records))


if __name__ == '__main__':
    unittest.main()
