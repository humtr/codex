#!/usr/bin/env python3
"""Independent optional Provider signature, flat-asset and HTTPS readback gate.

No signing key, model request, installation or Git ref mutation is performed.
The caller supplies its admitted public key; candidate files cannot choose it.
"""
import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import time

FILES = ('Cargo.lock', 'NOTICE.md', 'codex-chatgpt-provider', 'licenses/bridge-LICENSE',
         'licenses/novnc-LICENSE.txt', 'licenses/proot-LICENSE',
         'licenses/upstream-codex-LICENSE', 'source-inputs.json')
CONTROL = ('payload.manifest', 'payload.sig')
DEPENDENCIES = 'native-core;managed=proot+xvnc+explicit-rootfs;viewer=optional-assets'


def read(path, limit):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or not 0 < info.st_size <= limit:
        raise ValueError('bounded regular Provider file required: ' + path.name)
    with path.open('rb') as stream:
        data = stream.read(limit + 1)
    if len(data) != info.st_size:
        raise ValueError('Provider file changed: ' + path.name)
    return data


def sha(data):
    return hashlib.sha256(data).hexdigest()


def decimal(value):
    if not re.fullmatch(r'[1-9][0-9]*', value) or int(value) >= 2**64:
        raise ValueError('canonical positive sequence/size required')
    return int(value)


def parse(data):
    if b'\r' in data or not data.endswith(b'\n') or len(data) > 8192:
        raise ValueError('invalid Provider manifest bounds/text')
    rows = data.decode('utf-8').splitlines()
    keys = ('provider', 'generation_id', 'release_sequence', 'source', 'source_inputs_sha256',
            'target', 'core_api', 'dependencies', 'file_count')
    if len(rows) != 18 or rows[0] != 'codex-provider-payload-v1':
        raise ValueError('invalid Provider manifest shape')
    fields = {}
    for key, row in zip(keys, rows[1:10], strict=True):
        parts = row.split('\t')
        if len(parts) != 2 or parts[0] != key:
            raise ValueError('invalid ordered Provider field')
        fields[key] = parts[1]
    if (fields['provider'] != 'chatgpt' or not re.fullmatch(r'chatgpt-[a-z0-9](?:[a-z0-9-]{0,111}[a-z0-9])?', fields['generation_id'])
            or len(fields['generation_id']) > 120 or not re.fullmatch('[0-9a-f]{40}', fields['source'])
            or not re.fullmatch('[0-9a-f]{64}', fields['source_inputs_sha256'])
            or fields['target'] != 'aarch64-linux-android' or fields['core_api'] != 'provider-payload-v1'
            or fields['dependencies'] != DEPENDENCIES or fields['file_count'] != '8'):
        raise ValueError('unsupported Provider contract/provenance')
    decimal(fields['release_sequence'])
    entries = {}
    for name, row in zip(FILES, rows[10:], strict=True):
        parts = row.split('\t')
        mode = '0755' if name == 'codex-chatgpt-provider' else '0644'
        if (len(parts) != 5 or parts[:2] != ['file', name] or not re.fullmatch('[0-9a-f]{64}', parts[2])
                or parts[4] != mode or decimal(parts[3]) > (128 * 1024 * 1024 if mode == '0755' else 1024 * 1024)):
            raise ValueError('invalid Provider file inventory')
        entries[name] = dict(sha256=parts[2], size=int(parts[3]), mode=int(mode, 8))
    if entries['source-inputs.json']['sha256'] != fields['source_inputs_sha256']:
        raise ValueError('Provider source inventory binding mismatch')
    return fields, entries


def signature(package, public_key, openssl):
    # Validate key algorithm explicitly, then verify detached signature.
    key = subprocess.check_output([str(openssl), 'pkey', '-pubin', '-in', str(public_key), '-pubout', '-outform', 'DER'], stderr=subprocess.DEVNULL)
    if len(key) != 44 or not key.startswith(bytes.fromhex('302a300506032b6570032100')):
        raise ValueError('admitted Provider authority must be Ed25519')
    if len(read(package / 'payload.sig', 64)) != 64:
        raise ValueError('invalid Provider signature length')
    subprocess.run([str(openssl), 'pkeyutl', '-verify', '-rawin', '-pubin', '-inkey', str(public_key),
                    '-in', str(package / 'payload.manifest'), '-sigfile', str(package / 'payload.sig')],
                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                   env={}, check=True, timeout=15)


def verify(package, public_key, openssl):
    if package.is_symlink() or not package.is_dir():
        raise ValueError('real Provider package directory required')
    expected = {*FILES, *CONTROL, 'licenses'}
    if {str(p.relative_to(package)) for p in package.rglob('*')} != expected:
        raise ValueError('Provider inventory incomplete or extended')
    if (package / 'licenses').is_symlink() or not (package / 'licenses').is_dir():
        raise ValueError('real Provider licenses directory required')
    manifest = read(package / 'payload.manifest', 8192)
    fields, entries = parse(manifest)
    signature(package, public_key, openssl)
    for name, entry in entries.items():
        path = package / name
        data = read(path, entry['size'])
        if len(data) != entry['size'] or sha(data) != entry['sha256'] or stat.S_IMODE(path.stat().st_mode) != entry['mode']:
            raise ValueError('Provider payload hash/size/mode mismatch: ' + name)
    binary = read(package / 'codex-chatgpt-provider', 128 * 1024 * 1024)
    if len(binary) < 64 or binary[:6] != b'\x7fELF\x02\x01' or binary[18:20] != b'\xb7\0':
        raise ValueError('Provider executable is not Android/AArch64')
    return fields


def export(package, output, public_key, openssl):
    fields = verify(package, public_key, openssl)
    output.mkdir(mode=0o700)
    for name in (*FILES, *CONTROL):
        shutil.copy2(package / name, output / name.replace('/', '--'))
    return fields


def restore(assets, output, public_key, openssl):
    if assets.is_symlink() or {p.name for p in assets.iterdir()} != {name.replace('/', '--') for name in (*FILES, *CONTROL)}:
        raise ValueError('flat Provider asset inventory mismatch')
    fields, entries = parse(read(assets / 'payload.manifest', 8192))
    output.mkdir(mode=0o700)
    (output / 'licenses').mkdir(mode=0o700)
    for name in (*FILES, *CONTROL):
        bound = entries[name]['size'] if name in entries else (8192 if name == 'payload.manifest' else 64)
        data = read(assets / name.replace('/', '--'), bound)
        target = output / name
        with target.open('xb') as stream:
            stream.write(data)
        target.chmod(entries[name]['mode'] if name in entries else 0o644)
    return verify(output, public_key, openssl)


def parse_index(data):
    if len(data)>4096 or b'\r' in data or not data.endswith(b'\n'):
        raise ValueError('invalid Provider index bounds/text')
    rows=data.decode().splitlines()
    keys=('provider','channel','generation_id','release_sequence','release_base','manifest_sha256')
    if len(rows)!=7 or rows[0]!='codex-provider-index-v1':
        raise ValueError('invalid Provider index shape')
    fields={}
    for key,row in zip(keys,rows[1:],strict=True):
        parts=row.split('\t')
        if len(parts)!=2 or parts[0]!=key:raise ValueError('invalid ordered Provider index field')
        fields[key]=parts[1]
    if (fields['provider']!='chatgpt' or fields['channel']!='stable'
        or not re.fullmatch(r'chatgpt-[a-z0-9](?:[a-z0-9-]{0,111}[a-z0-9])?',fields['generation_id'])
        or fields['release_base']!='https://github.com/humtr/codex/releases/download/'+fields['generation_id']+'/'
        or not re.fullmatch('[0-9a-f]{64}',fields['manifest_sha256'])):
        raise ValueError('invalid Provider index identity/binding')
    decimal(fields['release_sequence'])
    return fields


def verify_index_record(index, signature_path, public_key, openssl):
    fields=parse_index(read(index,4096))
    if len(read(signature_path,64))!=64:raise ValueError('invalid Provider index signature')
    subprocess.run([str(openssl),'pkeyutl','-verify','-rawin','-pubin','-inkey',str(public_key),'-in',str(index),'-sigfile',str(signature_path)],check=True,env={},stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,timeout=15)
    return fields


def verify_index(index, signature_path, package, public_key, openssl):
    fields = verify(package, public_key, openssl)
    verify_index_record(index, signature_path, public_key, openssl)
    expected = ('codex-provider-index-v1\nprovider\tchatgpt\nchannel\tstable\ngeneration_id\t' + fields['generation_id'] +
                '\nrelease_sequence\t' + fields['release_sequence'] + '\nrelease_base\thttps://github.com/humtr/codex/releases/download/' +
                fields['generation_id'] + '/\nmanifest_sha256\t' + sha(read(package / 'payload.manifest', 8192)) + '\n')
    if read(index, 4096) != expected.encode() or len(read(signature_path, 64)) != 64:
        raise ValueError('Provider stable index/package binding mismatch')
    subprocess.run([str(openssl), 'pkeyutl', '-verify', '-rawin', '-pubin', '-inkey', str(public_key),
                    '-in', str(index), '-sigfile', str(signature_path)], check=True, env={},
                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=15)
    return fields


def readback(url, output, public_key, openssl, curl):
    # Only a versioned immutable GitHub release is a publication destination.
    if not re.fullmatch(r'https://github\.com/humtr/codex/releases/download/chatgpt-[a-z0-9](?:[a-z0-9-]{0,111}[a-z0-9])?/', url):
        raise ValueError('immutable owned Provider release URL required')
    output.mkdir(mode=0o700)
    deadline = time.monotonic() + 240
    def fetch(name, bound):
        remaining = int(deadline - time.monotonic())
        if remaining <= 0:
            raise ValueError('Provider readback deadline exhausted')
        subprocess.run([str(curl), '--disable', '--fail', '--silent', '--show-error', '--location',
                        '--proto', '=https', '--proto-redir', '=https', '--connect-timeout', '15',
                        '--max-time', str(min(remaining, 120)), '--max-filesize', str(bound),
                        '--output', str(output / name.replace('/', '--')), '--url', url + name.replace('/', '--')],
                       check=True, stdin=subprocess.DEVNULL, timeout=remaining + 2)
    fetch('payload.manifest', 8192)
    fetch('payload.sig', 64)
    fields, entries = parse(read(output / 'payload.manifest', 8192))
    signature(output, public_key, openssl)
    if url != f"https://github.com/humtr/codex/releases/download/{fields['generation_id']}/":
        raise ValueError('public Provider URL/identity mismatch')
    for name, entry in entries.items():
        fetch(name, entry['size'])
    fetch('provider-index-v1', 4096)
    fetch('provider-index-v1.sig', 64)
    with tempfile.TemporaryDirectory(prefix='provider-readback-', dir=output.parent) as temporary:
        temporary = Path(temporary)
        assets = temporary / 'assets'
        assets.mkdir()
        for name in (*FILES, *CONTROL):
            shutil.copy2(output / name.replace('/', '--'), assets / name.replace('/', '--'))
        package = temporary / 'package'
        restore(assets, package, public_key, openssl)
        return verify_index(output / 'provider-index-v1', output / 'provider-index-v1.sig', package, public_key, openssl)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('verify', 'export', 'restore', 'readback'))
    parser.add_argument('--package', type=Path)
    parser.add_argument('--assets', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--url')
    parser.add_argument('--public-key', type=Path, required=True)
    parser.add_argument('--openssl', type=Path, default=Path(shutil.which('openssl')))
    parser.add_argument('--curl', type=Path, default=Path(shutil.which('curl')))
    args = parser.parse_args()
    if args.action == 'verify':
        fields = verify(args.package, args.public_key, args.openssl)
    elif args.action == 'export':
        fields = export(args.package, args.output, args.public_key, args.openssl)
    elif args.action == 'restore':
        fields = restore(args.assets, args.output, args.public_key, args.openssl)
    else:
        fields = readback(args.url, args.output, args.public_key, args.openssl, args.curl)
    print('verified_provider=' + fields['generation_id'])
    print('verified_sequence=' + fields['release_sequence'])


if __name__ == '__main__':
    main()
