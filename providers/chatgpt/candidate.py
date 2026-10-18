"""Create/read back an unsigned Android qualification snapshot; never activate it."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess

BINARY = 'codex-chatgpt-provider'
SOURCE_FILES = ('Cargo.lock', 'NOTICE.md', 'licenses/bridge-LICENSE',
                'licenses/novnc-LICENSE.txt', 'licenses/proot-LICENSE',
                'licenses/upstream-codex-LICENSE')
FILES = {BINARY, *SOURCE_FILES}
FLAGS = ('signed', 'device_qualified', 'installation_authorized_by_this_record',
         'app_rootfs_or_credentials_bundled')


def read(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as stream:
        metadata = os.fstat(stream.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > limit:
            raise ValueError('candidate asset is nonregular or too large')
        data = stream.read(limit + 1)
        if len(data) > limit:
            raise ValueError('candidate asset is too large')
        return data


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate candidate metadata key')
        result[key] = value
    return result


def source_bytes(source, sha, name):
    return subprocess.check_output(
        ['git', '-C', str(source), 'show', f'{sha}:providers/chatgpt/{name}'],
        stderr=subprocess.DEVNULL)


def verify(artifact, source, sha):
    if not re.fullmatch('[0-9a-f]{40}', sha):
        raise ValueError('exact source commit required')
    if artifact.is_symlink() or not artifact.is_dir():
        raise ValueError('candidate directory required')
    paths = set()
    for path in artifact.rglob('*'):
        name = path.relative_to(artifact).as_posix()
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode) and name == 'licenses':
            continue
        if not stat.S_ISREG(mode):
            raise ValueError('nonregular candidate entry')
        paths.add(name)
    if paths != FILES | {'candidate.json'}:
        raise ValueError('candidate inventory incomplete or extended')
    record = json.loads(read(artifact / 'candidate.json', 64 * 1024),
                        object_pairs_hook=unique_object)
    expected = {'schema', 'source', 'run_id', 'target', 'api', 'ndk', 'rust',
                'files', *FLAGS}
    if (not isinstance(record, dict) or set(record) != expected
            or record['schema'] != 'termux-codex-provider-candidate-v1'
            or record['source'] != sha
            or record['target'] != 'aarch64-linux-android' or record['api'] != 24
            or any(record[key] is not False for key in FLAGS)
            or not isinstance(record['files'], dict) or set(record['files']) != FILES
            or any(not isinstance(record[key], str)
                   or not re.fullmatch('[A-Za-z0-9_.-]{1,96}', record[key])
                   for key in ('run_id', 'ndk', 'rust'))):
        raise ValueError('candidate source/build boundary mismatch')
    for name in sorted(FILES):
        data = read(artifact / name, 256 * 1024 * 1024 if name == BINARY else 1024 * 1024)
        inventory = {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
        if record['files'][name] != inventory:
            raise ValueError('candidate content mismatch: ' + name)
        if name == BINARY:
            # ELF64 little-endian AArch64. Execution/ABI qualification is separate.
            if len(data) < 64 or data[:6] != b'\x7fELF\x02\x01' or data[18:20] != b'\xb7\x00':
                raise ValueError('candidate is not an AArch64 ELF64 executable')
        elif data != source_bytes(source, sha, name):
            raise ValueError('candidate asset differs from exact source: ' + name)
    return record


def package(binary, artifact, source, sha, run_id, ndk, rust):
    if (not re.fullmatch('[0-9a-f]{40}', sha)
            or subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD']).decode().strip() != sha
            or subprocess.check_output(['git', '-C', str(source), 'status', '--porcelain',
                                       '--untracked-files=normal', '--', 'providers/chatgpt',
                                       '.github/workflows/provider-qualification.yml'])):
        raise ValueError('clean exact Provider source required')
    data = read(binary, 256 * 1024 * 1024)
    artifact.mkdir(mode=0o700)  # Create-new only, including on failed/partial retries.
    (artifact / 'licenses').mkdir()
    (artifact / BINARY).write_bytes(data)
    (artifact / BINARY).chmod(0o755)
    for name in SOURCE_FILES:
        (artifact / name).write_bytes(source_bytes(source, sha, name))
    files = {}
    for name in sorted(FILES):
        data = (artifact / name).read_bytes()
        files[name] = {'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)}
    record = dict(schema='termux-codex-provider-candidate-v1', source=sha,
                  run_id=run_id, target='aarch64-linux-android', api=24,
                  ndk=ndk, rust=rust, files=files, **dict.fromkeys(FLAGS, False))
    (artifact / 'candidate.json').write_text(json.dumps(record, indent=2) + '\n')
    return verify(artifact, source, sha)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('package', 'verify'))
    parser.add_argument('--artifact', required=True, type=Path)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--source-sha', required=True)
    parser.add_argument('--binary', type=Path)
    for name in ('run-id', 'ndk', 'rust'):
        parser.add_argument('--' + name)
    args = parser.parse_args()
    try:
        if args.action == 'package':
            if any(getattr(args, key) is None for key in ('binary', 'run_id', 'ndk', 'rust')):
                parser.error('package requires --binary, --run-id, --ndk and --rust')
            record = package(args.binary, args.artifact, args.source, args.source_sha,
                             args.run_id, args.ndk, args.rust)
        else:
            record = verify(args.artifact, args.source, args.source_sha)
        print(json.dumps({'source': record['source'], 'files_verified': len(FILES),
                          'sha256': record['files'][BINARY]['sha256'], 'signed': False}))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, 'candidate refused: ' + str(error) + '\n')


if __name__ == '__main__':
    main()
