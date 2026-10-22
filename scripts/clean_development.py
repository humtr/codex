#!/usr/bin/env python3
"""Reclaim owned compiler outputs and archived, inactive native fixture payloads.

Dry-run unless --apply is supplied. Never removes a fixture tree or user state.
Restore one removed payload by its exact path and the retained JSONL record.
"""
import argparse
import contextlib
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import time

TMP = Path(os.environ.get('TMPDIR', '/data/data/com.termux/files/usr/tmp')).resolve()
CACHE = Path(os.environ.get('XDG_CACHE_HOME', str(Path.home() / '.cache'))) / 'codex'
PAYLOADS = ('runtime', 'codex-code-mode-host', 'helpers/2')


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def owned(path, directory=False):
    if not path.is_absolute() or path.resolve() != path:
        raise ValueError('canonical owned path required')
    info = path.lstat()
    if info.st_uid != os.getuid() or not (stat.S_ISDIR(info.st_mode) if directory
                                        else stat.S_ISREG(info.st_mode)):
        raise ValueError('owned regular file/directory required')
    return info


def identity(path):
    info = owned(path)
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def process_references():
    """Return paths only, never credential-bearing argv/environment contents."""
    refs = []
    compilers = []
    for proc in Path('/proc').iterdir():
        if not proc.name.isdecimal():
            continue
        try:
            if proc.stat().st_uid != os.getuid():
                continue
            name = (proc / 'comm').read_text().strip()
            if name in ('cargo', 'rustc', 'clippy-driver', 'rustfmt'):
                compilers.append(int(proc.name))
            for item in ('exe', 'cwd', 'root'):
                try:
                    refs.append(os.readlink(proc / item).removesuffix(' (deleted)'))
                except OSError:
                    pass
            try:
                for entry in (proc / 'fd').iterdir():
                    try:
                        refs.append(os.readlink(entry).removesuffix(' (deleted)'))
                    except OSError:
                        pass
            except OSError:
                pass
            try:
                for line in (proc / 'maps').read_text().splitlines():
                    fields = line.split(None, 5)
                    if len(fields) == 6 and fields[5].startswith('/'):
                        refs.append(fields[5].removesuffix(' (deleted)'))
            except OSError:
                pass
            try:
                # Our --root/--path names the maintenance target, not a running
                # consumer. Keep our cwd/open-file/mapping checks above.
                if int(proc.name) != os.getpid():
                    args = (proc / 'cmdline').read_bytes().decode(errors='replace')
                    refs.extend(re.findall(re.escape(str(TMP)) + r'/[^\s\x00\"\x27;:]+', args))
                for entry in (proc / 'environ').read_bytes().split(b'\0'):
                    name, _, value = entry.partition(b'=')
                    if name in (b'HOME', b'CODEX_HOME', b'XDG_STATE_HOME'):
                        refs.append(value.decode(errors='replace'))
            except OSError:
                pass
        except OSError:
            continue
    return refs, compilers


def busy(path, refs):
    prefix = str(path)
    return any(value == prefix or value.startswith(prefix + '/') for value in refs)


def log(record, value):
    with record.open('a') as stream:
        stream.write(json.dumps(value, sort_keys=True) + '\n')
        stream.flush()
        os.fsync(stream.fileno())


def cache_cleanup(report, apply, root=None):
    root = root or CACHE / 'provider-target'
    if not root.is_relative_to(CACHE) or root == CACHE or not (root / 'CACHEDIR.TAG').is_file():
        raise ValueError('an owned Cargo target cache with CACHEDIR.TAG is required')
    owned(root, directory=True)
    debug = root / 'debug'
    owned(debug, directory=True)
    refs, compilers = process_references()
    if compilers or busy(root, refs):
        raise ValueError('compiler or direct cache user is active; cache preserved')
    if not apply:
        for path in sorted(debug.iterdir()):
            if path.name not in ('.cargo-lock', '.cargo-artifact-lock', '.cargo-build-lock'):
                log(report, {'kind': 'compiler_output', 'path': str(path), 'removed': False})
        return
    with contextlib.ExitStack() as locks:
        lock_names = ('.cargo-lock', '.cargo-artifact-lock', '.cargo-build-lock')
        for name in lock_names:
            lock_path = debug / name
            if lock_path.exists():
                owned(lock_path)
            lock = locks.enter_context(lock_path.open('a+'))
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        # Keep all Cargo lock inodes; other entries are generated build outputs.
        for path in sorted(debug.iterdir()):
            if path.name in lock_names:
                continue
            row = {'kind': 'compiler_output', 'path': str(path), 'removed': False}
            if apply:
                owned(debug, directory=True)
                if path.is_dir() and not path.is_symlink():
                    shutil.rmtree(path)
                else:
                    path.unlink()
                row['removed'] = True
            log(report, row)


def generations():
    for root in sorted(TMP.iterdir()):
        if not root.is_dir() or root.is_symlink():
            continue
        cases = [root]
        if root.name.startswith(('g-product-', 'g-package-')):
            cases += [root / name for name in ('base', 'manager-no-tui', 'full')]
        if root.name.startswith('gpub.'):
            cases.append(root / 'consumer')
        if root.name.startswith('gp.'):
            cases.append(root / 'c')
        for case in cases:
            if not (case / 'fixture-key.pem').is_file():
                continue
            for generation in (case / 'local-permission-fixture',
                               case / 'h/.local/lib/codex/core/generations/local-permission-fixture'):
                if (generation / 'generation.meta').is_file():
                    yield root, generation


def inventory(generation):
    owned(generation, directory=True)
    metadata = (generation / 'generation.meta').read_text().splitlines()
    if metadata[0] != 'codex-local-generation-v2' or 'generation_id\tlocal-permission-fixture' not in metadata:
        raise ValueError('not a generated qualification fixture')
    manifest = (generation / 'release.manifest').read_text().splitlines()
    if manifest[0] not in ('codex-release-v3', 'codex-release-v4'):
        raise ValueError('unsupported generated manifest')
    result = {}
    for line in manifest:
        fields = line.split('\t')
        if len(fields) == 4 and fields[0] == 'file' and fields[1] in PAYLOADS:
            if not re.fullmatch('[0-9a-f]{64}', fields[2]) or fields[1] in result:
                raise ValueError('invalid payload inventory')
            result[fields[1]] = fields[2]
    return result


def archive(path, digest, objects, verified):
    blob = objects / (digest + '.zst')
    if digest in verified:
        return blob
    if not blob.exists():
        fd, name = tempfile.mkstemp(prefix='.compress-', dir=objects)
        temporary = Path(name)
        try:
            with os.fdopen(fd, 'wb') as output:
                subprocess.run(['zstd', '-q', '-3', '-T1', '-c', str(path)],
                               stdout=output, check=True, timeout=120)
                output.flush()
                os.fsync(output.fileno())
            temporary.rename(blob)
        finally:
            temporary.unlink(missing_ok=True)
    owned(blob)
    decoder = subprocess.Popen(['zstd', '-q', '-d', '-c', str(blob)], stdout=subprocess.PIPE)
    try:
        observed = hashlib.file_digest(decoder.stdout, 'sha256').hexdigest()
        decoder.stdout.close()
        assert decoder.wait(timeout=120) == 0 and observed == digest, 'archive readback mismatch'
    finally:
        if decoder.poll() is None:
            decoder.kill()
            decoder.wait()
    verified.add(digest)
    return blob


def fixture_cleanup(report, apply, only_root=None):
    objects = CACHE / 'acceptance/fixture-payloads/objects'
    if apply:
        objects.mkdir(parents=True, mode=0o700, exist_ok=True)
        owned(objects, directory=True)
    refs, _ = process_references()
    verified = set()
    removed = count = 0
    for root, generation in generations():
        if only_root is not None and root != only_root:
            continue
        if busy(root, refs):
            continue
        try:
            entries = inventory(generation)
        except (OSError, ValueError, IndexError):
            continue
        for name, expected in entries.items():
            path = generation / name
            if not path.exists():
                continue
            before = identity(path)
            if not apply:
                count += 1
                continue
            # Recheck the complete top-level fixture, including native homes.
            latest, _ = process_references()
            if busy(root, latest):
                break
            if sha(path) != expected or identity(path) != before:
                log(report, {'kind': 'preserved_changed_payload', 'path': str(path)})
                continue
            info = path.stat()
            blob = archive(path, expected, objects, verified)
            row = dict(kind='fixture_payload', path=str(path), sha256=expected,
                       archive=str(blob), bytes=info.st_size, blocks=info.st_blocks,
                       mode=stat.S_IMODE(info.st_mode), mtime_ns=info.st_mtime_ns,
                       removed=False)
            log(report, row)
            latest, _ = process_references()
            if busy(root, latest) or identity(path) != before:
                continue
            path.unlink()
            row['removed'] = True
            log(report, row)
            removed += info.st_blocks * 512
            count += 1
            if count % 20 == 0:
                print(json.dumps({'payloads_removed': count, 'GiB': round(removed / 2**30, 2)}), flush=True)
    return {'payloads': count, 'removed_allocated_bytes': removed, 'objects_verified': len(verified)}


def restore(path, record):
    path = Path(path)
    if not path.is_relative_to(TMP) or path.resolve() != path:
        raise ValueError('exact original TMPDIR payload path required')
    rows = [json.loads(line) for line in record.read_text().splitlines()]
    matches = [row for row in rows if row.get('kind') == 'fixture_payload'
               and row.get('path') == str(path) and row.get('removed') is True]
    if not matches:
        raise ValueError('no completed removal record for this path')
    row = matches[-1]
    if path.exists():
        assert sha(path) == row['sha256'], 'existing content preserved'
        return
    owned(path.parent, directory=True)
    refs, _ = process_references()
    root = TMP / path.relative_to(TMP).parts[0]
    if busy(root, refs):
        raise ValueError('fixture is active; do not restore during execution')
    blob = Path(row['archive'])
    expected_parent = CACHE / 'acceptance/fixture-payloads/objects'
    if blob.parent != expected_parent or blob.name != row['sha256'] + '.zst':
        raise ValueError('invalid archive reference')
    owned(blob)
    # Create-new prevents overwriting a new user file. Execute permission is
    # restored only after complete decompression and original-hash verification.
    with path.open('xb') as output:
        os.fchmod(output.fileno(), 0o600)
        subprocess.run(['zstd', '-q', '-d', '-c', str(blob)], stdout=output, check=True, timeout=120)
        output.flush()
        os.fsync(output.fileno())
    if sha(path) != row['sha256']:
        raise ValueError('restored payload mismatch; retained for inspection')
    path.chmod(row['mode'])
    os.utime(path, ns=(row['mtime_ns'], row['mtime_ns']))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('cache', 'fixtures', 'restore'))
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--record', type=Path, required=True)
    parser.add_argument('--path', type=Path)
    parser.add_argument('--root', type=Path, help='limit fixture cleanup to one exact TMPDIR child')
    parser.add_argument('--cache-root', type=Path, help='explicit Cargo target under the Codex cache')
    args = parser.parse_args()
    os.umask(0o077)
    if args.action == 'restore':
        if not args.path or not args.apply:
            parser.error('restore requires --path and --apply')
        restore(args.path, args.record)
        print('Original payload restored; no process or model request started.')
        return
    if args.record.exists():
        parser.error('record exists; preserve it and use a distinct cleanup record')
    owned(args.record.parent, directory=True)
    started = time.monotonic()
    before = os.statvfs(CACHE)
    log(args.record, {'kind': 'start', 'action': args.action, 'apply': args.apply,
                      'driver_sha256': sha(Path(__file__).resolve()),
                      'UTC': datetime.datetime.now(datetime.timezone.utc).isoformat()})
    if args.root:
        owned(args.root, directory=True)
        if args.root.parent != TMP or args.action != 'fixtures':
            parser.error('--root requires one direct TMPDIR fixture directory')
    if args.cache_root and args.action != 'cache':
        parser.error('--cache-root requires cache action')
    result = cache_cleanup(args.record, args.apply, args.cache_root) if args.action == 'cache' else fixture_cleanup(args.record, args.apply, args.root)
    after = os.statvfs(CACHE)
    row = dict(kind='complete', seconds=time.monotonic() - started, result=result,
               free_bytes_before=before.f_bavail * before.f_frsize,
               free_bytes_after=after.f_bavail * after.f_frsize)
    log(args.record, row)
    print(json.dumps(row))


if __name__ == '__main__':
    main()
