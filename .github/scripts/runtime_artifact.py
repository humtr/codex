#!/usr/bin/env python3
"""Carry the common raw runtime through Actions without losing file modes.

Run/artifact authentication belongs to the workflow. The Rust verifier owns
the runtime inventory, ELF, digests and source/dependency admission. This module
only wraps those admitted files in a bounded flat tar and restores its modes.
"""
import argparse
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tarfile

ARCHIVE = 'runtime.tar'
MAX_ARCHIVE = 1024 * 1024 * 1024
MAX_MEMBERS = 32


def verify(artifact, builder, recipe, openssl):
    subprocess.run([str(builder), 'verify-runtime', '--artifact', str(artifact),
                    '--recipe', str(recipe), '--openssl', str(openssl)], check=True)


def directory(path):
    if not stat.S_ISDIR(path.lstat().st_mode):
        raise ValueError('runtime transfer root must be a real directory')


def regular(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    stream = os.fdopen(descriptor, 'rb')
    if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
        stream.close()
        raise ValueError('runtime transfer input must be a regular file')
    return stream


def member(name, mode, size):
    if (not re.fullmatch(r'[A-Za-z0-9_.-]{1,128}', name) or name in {'.', '..'}
            or mode & ~0o777 or mode & 0o022 or size < 0 or size > MAX_ARCHIVE):
        raise ValueError('runtime archive member path, mode or size is unsafe')


def pack(artifact, transfer, builder, recipe, openssl):
    """Verify producer output, then create one permission-preserving archive."""
    artifact, transfer = Path(artifact), Path(transfer)
    verify(artifact, builder, recipe, openssl)
    transfer.mkdir(mode=0o700)
    try:
        paths = sorted(artifact.iterdir())
        if len(paths) > MAX_MEMBERS:
            raise ValueError('runtime archive member count exceeds its bound')
        with (transfer / ARCHIVE).open('xb') as output:
            with tarfile.open(fileobj=output, mode='w', format=tarfile.USTAR_FORMAT) as archive:
                total = 0
                for path in paths:
                    with regular(path) as source:
                        before = os.fstat(source.fileno())
                        mode = stat.S_IMODE(before.st_mode)
                        member(path.name, mode, before.st_size)
                        total += before.st_size
                        if total > MAX_ARCHIVE:
                            raise ValueError('runtime archive exceeds its byte bound')
                        entry = tarfile.TarInfo(path.name)
                        entry.mode, entry.size = mode, before.st_size
                        archive.addfile(entry, source)
                        after = os.fstat(source.fileno())
                        if (before.st_size, before.st_mtime_ns, before.st_mode) != (
                                after.st_size, after.st_mtime_ns, after.st_mode):
                            raise ValueError('runtime artifact changed during transfer')
            if output.tell() > MAX_ARCHIVE:
                raise ValueError('runtime archive exceeds its byte bound')
    except BaseException:
        shutil.rmtree(transfer)
        raise


def unpack(transfer, artifact, builder, recipe, openssl):
    """Restore a downloaded transfer, then require the unchanged Rust admission.

    Callers must first authenticate the selected workflow/run/artifact. An old
    loose artifact is not guessed into this format and never chmod-repaired.
    The output must be absent; a failure removes only this call's new directory.
    """
    transfer, artifact = Path(transfer), Path(artifact)
    directory(transfer)
    if {path.name for path in transfer.iterdir()} != {ARCHIVE}:
        raise ValueError('runtime transfer inventory differs')
    with regular(transfer / ARCHIVE) as source:
        if os.fstat(source.fileno()).st_size > MAX_ARCHIVE:
            raise ValueError('runtime archive exceeds its byte bound')
        with tarfile.open(fileobj=source, mode='r:') as archive:
            entries, names, offset = [], set(), 0
            for entry in archive:
                member(entry.name, entry.mode, entry.size)
                if (len(entries) >= MAX_MEMBERS or entry.name in names
                        or entry.type not in {tarfile.REGTYPE, tarfile.AREGTYPE}
                        or entry.linkname or entry.pax_headers
                        or entry.offset != offset or entry.offset_data != offset + 512):
                    raise ValueError('runtime archive entries are not unique flat regular files')
                names.add(entry.name)
                entries.append(entry)
                offset = entry.offset_data + (entry.size + 511) // 512 * 512
                if offset > MAX_ARCHIVE:
                    raise ValueError('runtime archive exceeds its byte bound')
            source.seek(offset)
            ending = source.read(10241)
            if len(ending) < 1024 or len(ending) > 10240 or any(ending):
                raise ValueError('runtime archive end marker is invalid')
            artifact.mkdir(mode=0o700)
            try:
                # Do not delegate path creation, links or permissions to tar.
                for entry in entries:
                    with archive.extractfile(entry) as incoming:
                        with (artifact / entry.name).open('xb') as outgoing:
                            shutil.copyfileobj(incoming, outgoing)
                            if outgoing.tell() != entry.size:
                                raise ValueError('runtime archive payload is truncated')
                            os.fchmod(outgoing.fileno(), entry.mode)
                verify(artifact, builder, recipe, openssl)
            except BaseException:
                shutil.rmtree(artifact)
                raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('pack', 'unpack'))
    for flag in ('artifact', 'transfer', 'builder', 'recipe', 'openssl'):
        parser.add_argument('--' + flag, type=Path, required=True)
    args = parser.parse_args()
    try:
        {'pack': pack, 'unpack': unpack}[args.action](**{
            name: getattr(args, name) for name in ('artifact', 'transfer', 'builder', 'recipe', 'openssl')})
    except subprocess.CalledProcessError as error:
        raise SystemExit(error.returncode) from None
    except (OSError, ValueError, tarfile.TarError) as error:
        parser.exit(1, f'runtime artifact transfer: {error}\n')


if __name__ == '__main__':
    main()
