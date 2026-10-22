#!/usr/bin/env python3
"""Sign an explicitly qualified optional payload with the existing release secret."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

from rald4_signing import (SECRET_NAME, ed25519_raw_public, require_real_directory,
                           require_regular_file, write_private_key)
from provider_publication import verify


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def qualified(args):
    record = json.loads(args.qualification.read_text())
    if (record.get('schema') != 'termux-codex-provider-release-qualification-v1'
            or record.get('accepted') is not True
            or record.get('source_sha') != args.source_sha
            or record.get('binary_sha256') != sha(args.binary)
            or record.get('source_inputs_sha256') != sha(args.source_inputs)
            or not all(record.get('gates', {}).get(name) is True for name in
                       ('source', 'android', 'native_tools', 'continuity', 'stop_recovery',
                        'installed_compositions', 'signed_package_lifecycle', 'host_lifecycle'))):
        raise ValueError('complete exact-input Provider qualification required before signing')
    inputs = json.loads(args.source_inputs.read_text())
    files = inputs.get('files')
    if not isinstance(files, dict) or not files:
        raise ValueError('complete Provider source inventory required')
    root = args.source.resolve(strict=True)
    expected = {str(p.relative_to(root)) for directory in ('providers/chatgpt', 'crates/runtime-binding')
                for p in (root / directory).rglob('*') if p.is_file()
                and not {'target', '.git', '__pycache__'}.intersection(p.relative_to(root).parts)}
    if not expected <= files.keys():
        raise ValueError('Provider source inventory incomplete')
    for name, digest in files.items():
        path = Path(name)
        if path.is_absolute() or '..' in path.parts or path.as_posix() != name or sha(root / path) != digest:
            raise ValueError('qualified Provider source input drift')
    if subprocess.check_output(['git', '-C', root, 'rev-parse', 'HEAD']).decode().strip() != args.source_sha:
        raise ValueError('qualified source commit mismatch')


def sign(args):
    qualified(args)
    accepted = require_regular_file(args.public_key, 16384, 'accepted authority')
    openssl = require_regular_file(args.openssl, 128 * 1024**2, 'OpenSSL', executable=True)
    require_regular_file(args.builder, 128 * 1024**2, 'release builder', executable=True)
    temporary = require_real_directory(args.runner_temp, 'signing temporary root')
    authority = ed25519_raw_public(openssl, accepted, private=False)
    secret = os.environ.pop(SECRET_NAME, None)
    if not secret:
        raise ValueError('Provider signing secret unavailable')
    private_root = None
    try:
        private_root, private = write_private_key(temporary, secret)
        if ed25519_raw_public(openssl, private, private=True) != authority:
            raise ValueError('Provider signing secret does not match admitted authority')
        subprocess.run([str(args.builder), 'provider-package', '--binary', str(args.binary),
                        '--assets', str(args.source / 'providers/chatgpt'), '--source-inputs', str(args.source_inputs),
                        '--source-sha', args.source_sha, '--generation-id', args.generation,
                        '--release-sequence', args.sequence, '--private-key', str(private),
                        '--openssl', str(openssl), '--output', str(args.output)],
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                       env={}, check=True, timeout=120)
        fields = verify(args.output, accepted, openssl)
        # The stable index is signed separately and retained beside the payload.
        index = args.output.parent / 'provider-index-v1'
        text = ('codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\ngeneration_id\t' + args.generation +
                '\nrelease_sequence\t' + args.sequence + '\nrelease_base\thttps://github.com/humtr/codex/releases/download/' +
                args.generation + '/\nmanifest_sha256\t' + sha(args.output / 'payload.manifest') + '\n')
        with index.open('x') as stream:
            stream.write(text)
        subprocess.run([str(openssl), 'pkeyutl', '-sign', '-rawin', '-inkey', str(private), '-in', str(index), '-out', str(index) + '.sig'],
                       check=True, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env={}, timeout=15)
        if fields['generation_id'] != args.generation or fields['release_sequence'] != args.sequence:
            raise ValueError('signed Provider identity changed')
        return fields
    finally:
        if private_root is not None:
            shutil.rmtree(private_root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('source', 'binary', 'source-inputs', 'qualification', 'public-key', 'openssl',
                 'builder', 'runner-temp', 'output'):
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ('source-sha', 'generation', 'sequence'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    for name, value in vars(args).items():
        if isinstance(value, Path) and not value.is_absolute():
            parser.error(name + ' must be absolute')
    fields = sign(args)
    print('verified_provider=' + fields['generation_id'])


if __name__ == '__main__':
    main()
