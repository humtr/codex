#!/usr/bin/env python3
"""CI runtime build wiring for a prepared Linux host.

The executor owns tools and a disposable pristine checkout. This producer owns
adaptation, dependency audit, compilation and the existing raw artifact contract;
it grants no signing, installation or publication authority.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

SOURCE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SOURCE / '.github/scripts'))
from upstream_release import load

TARGET = 'aarch64-unknown-linux-musl'
# This is the accepted size-oriented runtime recipe. Executors may choose Cargo
# parallelism; compilation semantics remain the same on a local or remote host.
BUILD = {'CARGO_PROFILE_RELEASE_OPT_LEVEL': 'z',
         'CARGO_PROFILE_RELEASE_CODEGEN_UNITS': '1',
         'CARGO_PROFILE_RELEASE_STRIP': 'symbols',
         'CARGO_PROFILE_RELEASE_DEBUG': '0',
         'CARGO_PROFILE_RELEASE_INCREMENTAL': 'false'}
ASSET_MANIFEST = 'release/runtime-build-inputs-v1'


def assets(contents):
    lines = contents.splitlines()
    if not lines or lines[0] != 'codex-runtime-build-inputs-v1':
        raise ValueError('unsupported runtime build input inventory')
    result = {}
    for line in lines[1:]:
        role, separator, name = line.partition('\t')
        path = Path(name)
        if (not separator or role not in {'recipe', 'input'} or name in result
                or path.is_absolute() or any(part in {'', '.', '..'} for part in name.split('/'))):
            raise ValueError('invalid runtime build input record')
        result[name] = role
    return result


INPUT_FILES = tuple(name for name, role in assets((SOURCE / ASSET_MANIFEST).read_text()).items()
                    if role == 'recipe')
ARTIFACT_FILES = {'codex', 'version.txt', 'sha256.txt', 'preparation.json',
                  'lock-audit.json', 'source.patch'}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def cargo_dependencies(read, manifest):
    """The exact locked dependency closure of one local build tool/component."""
    lock = tomllib.loads(read('Cargo.lock').decode())['package']
    packages = {(p['name'], p['version'], p.get('source')): p for p in lock}
    if len(packages) != len(lock):
        raise ValueError('duplicate Cargo build package identity')
    root = tomllib.loads(read(manifest).decode())['package']
    pending = [(root['name'], root['version'], None)]
    selected = {}
    while pending:
        key = pending.pop()
        if key in selected:
            continue
        package = packages[key]
        selected[key] = package
        for dependency in package.get('dependencies', []):
            description, separator, origin = dependency.partition(' (')
            if separator and not origin.endswith(')'):
                raise ValueError('invalid Cargo build dependency source')
            parts = description.split()
            if len(parts) not in (1, 2):
                raise ValueError('invalid Cargo build dependency identity')
            matches = [candidate for candidate in packages if candidate[0] == parts[0]
                       and (len(parts) == 1 or candidate[1] == parts[1])
                       and (not separator or candidate[2] == origin[:-1])]
            if len(matches) != 1:
                raise ValueError('ambiguous Cargo build dependency closure')
            pending.append(matches[0])
    return selected


def locked_packages(selected):
    return sorted(selected.values(), key=lambda p: (p['name'], p['version'], p.get('source', '')))


def fingerprint(source, revision=None, recipe=None, adapted=False):
    """Build inputs, independent of a caller commit or unrelated Core/Manager code."""
    def read(name):
        if revision is not None:
            return subprocess.check_output(['git', '-C', str(source), 'show', revision + ':' + name],
                                           stderr=subprocess.DEVNULL)
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError('runtime build input is not a regular source file: ' + name)
        return path.read_bytes()
    value = recipe if recipe is not None else json.loads(read('release/upstream.json'))
    # Frontend run IDs and output digests are results, not runtime build inputs.
    inputs = {key: value[key] for key in ('upstream',)}
    inputs.update(rust=value['toolchains']['frontend_rust'],
                  lock_audit=value['qualification']['lock_audit'], target=TARGET,
                  build=BUILD, files={name: hashlib.sha256(read(name)).hexdigest()
                                      for name, role in assets(read(ASSET_MANIFEST).decode()).items() if role == 'recipe'})
    inputs['producer_dependencies'] = locked_packages(cargo_dependencies(read, 'crates/release-builder/Cargo.toml'))
    if adapted:
        # Published runtime bytes also consume the Rust adaptation/descriptor
        # rules. Reusing an already-adapted Base must bind those crate inputs,
        # while the raw compiler cache remains independent of adapter changes.
        tree = subprocess.check_output(['git', '-C', str(source), 'ls-tree', '-r',
            revision or 'HEAD', '--', 'crates/release-builder', 'crates/runtime-binding'])
        inputs['adaptation'] = {'core_rust': value['toolchains']['core_rust'],
                               'sources': tree.decode().splitlines(),
                               'binding_dependencies': locked_packages(cargo_dependencies(read, 'crates/runtime-binding/Cargo.toml'))}
    return hashlib.sha256(canonical(inputs)).hexdigest()


def verify_artifact(directory, recipe):
    """Transfer inventory/content identity, distinct from downstream execution proof."""
    if ({p.name for p in directory.iterdir()} != ARTIFACT_FILES
            or any(p.is_symlink() or not p.is_file() for p in directory.iterdir())):
        raise ValueError('runtime artifact inventory differs')
    binary = directory / 'codex'
    with binary.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    if ((directory / 'sha256.txt').read_text() != digest + '  codex\n'
            or (directory / 'version.txt').read_text() != 'codex-cli ' + recipe['upstream']['version'] + '\n'):
        raise ValueError('runtime artifact digest or version differs')
    preparation = json.loads((directory / 'preparation.json').read_text())
    if (preparation['source'] != recipe['upstream']['source']
            or preparation['version'] != recipe['upstream']['version']
            or json.loads((directory / 'lock-audit.json').read_text()) != recipe['qualification']['lock_audit']):
        raise ValueError('runtime artifact source/dependency identity differs')
    return digest


def run(phase, command, **kwargs):
    print('[runtime] ' + phase, flush=True)
    return subprocess.run([str(arg) for arg in command], check=True, **kwargs)


def build(checkout, output, recipe_path):
    recipe = load(recipe_path)
    if not checkout.is_absolute() or checkout.is_symlink() or output.exists() or output.is_symlink():
        raise ValueError('runtime build requires an absolute disposable checkout and absent output')
    expected = recipe['upstream']['source']
    actual = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD'], text=True).strip()
    changes = subprocess.check_output(['git', '-C', str(checkout), 'status', '--porcelain'])
    if actual != expected or changes:
        raise ValueError('official runtime source differs or is not pristine; existing changes preserved')
    env = dict(os.environ, **BUILD, RUSTUP_TOOLCHAIN=recipe['toolchains']['frontend_rust'])
    version = subprocess.check_output(['rustc', '--version'], env=env, text=True).split()
    if len(version) < 2 or version[1] != recipe['toolchains']['frontend_rust']:
        raise ValueError('prepared Rust compiler does not match the selected runtime recipe')
    sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], env=env, text=True).strip())
    if not (sysroot / 'lib/rustlib' / TARGET / 'lib').is_dir():
        raise ValueError('prepared Rust compiler is missing the musl target')
    # Keep source/recipe cache identities honest about ambient build settings.
    overrides = {'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTFLAGS',
                 'CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS'}
    overrides.update(key for key in env if key.startswith('CARGO_PROFILE_RELEASE_') and key not in BUILD)
    for key in overrides:
        if env.get(key):
            raise ValueError('runtime build has an unrecorded compiler override: ' + key)
    env.setdefault('CARGO_BUILD_JOBS', '3')
    cache = Path(env.get('XDG_CACHE_HOME', str(Path.home() / '.cache')))
    env.setdefault('CARGO_TARGET_DIR', str(cache / 'codex/build/runtime' / fingerprint(SOURCE, recipe=recipe)))
    if not Path(env['CARGO_TARGET_DIR']).is_absolute():
        raise ValueError('runtime compiler cache must be an absolute path')
    output.mkdir(mode=0o700, parents=False)
    builder = Path(env.get('CODEX_RELEASE_BUILDER', str(SOURCE / 'target/release/codex-release-builder')))
    if not builder.is_absolute() or builder.is_symlink() or not builder.is_file():
        raise ValueError('prepared Rust release-builder is required')
    with (output / 'preparation.json').open('wb') as report:
        run('adapt exact official source', [builder, 'prepare-source', '--source',
            checkout, '--version', recipe['upstream']['version'], '--source-sha', expected],
            stdout=report, env=env)
    run('audit unchanged external dependencies', [builder, 'normalize-lock', '--source',
        checkout / 'codex-rs', '--openssl', shutil.which('openssl', path=env['PATH']),
        '--audit', output / 'lock-audit.json'], env=env)
    if json.loads((output / 'lock-audit.json').read_text()) != recipe['qualification']['lock_audit']:
        raise ValueError('source runtime dependency audit differs from exact release input')
    run('check source diff', ['git', '-C', checkout, 'diff', '--check'], env=env)
    with (output / 'source.patch').open('wb') as patch:
        run('retain source adaptation', ['git', '-C', checkout, 'diff', '--binary'], stdout=patch, env=env)
    run('preserve unchanged compiler inputs', [builder, 'preserve-source-times',
        '--source', checkout], env=env)
    run('compile adapted upstream executable', ['cargo', 'build', '--release', '--locked',
        '--target', TARGET, '-p', 'codex-cli', '--bin', 'codex'], cwd=checkout / 'codex-rs', env=env)
    artifact = Path(env['CARGO_TARGET_DIR']) / TARGET / 'release/codex'
    finish_artifact(artifact, output, recipe, env)


def finish_artifact(artifact, output, recipe, env):
    """Freeze shared compiler bytes, execute that copy, then seal raw transfer data."""
    # Snapshot the shared compiler output before inspecting or executing it;
    # another build may replace that shared path after Cargo releases its lock.
    # Android/Core admission is still a separate downstream gate.
    shutil.copy2(artifact, output / 'codex')
    artifact = output / 'codex'
    header = subprocess.check_output(['readelf', '-h', artifact], env=env)
    segments = subprocess.check_output(['readelf', '-l', artifact], env=env)
    if b'AArch64' not in header or b'INTERP' in segments:
        raise ValueError('runtime output must be a static AArch64 ELF')
    home = output / '.smoke-home'
    home.mkdir(mode=0o700)
    try:
        version = run('execute produced version', [artifact, '--version'],
            env={'HOME': str(home), 'CODEX_HOME': str(home / '.codex'), 'PATH': '/usr/bin:/bin'},
            capture_output=True, timeout=20)
        expected_version = 'codex-cli ' + recipe['upstream']['version'] + '\n'
        if version.stdout != expected_version.encode() or version.stderr:
            raise ValueError('source-built runtime version/streams differ')
        (output / 'version.txt').write_bytes(version.stdout)
    finally:
        shutil.rmtree(home)  # This invocation created the isolated smoke home.
    with artifact.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    (output / 'sha256.txt').write_text(digest + '  codex\n')
    verify_artifact(output, recipe)
    print('[runtime] raw artifact complete; Android admission is still required', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, help='disposable pristine exact official checkout')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--recipe', type=Path, default=SOURCE / 'release/upstream.json')
    parser.add_argument('--fingerprint', action='store_true', help='print current producer inputs without executing a build')
    args = parser.parse_args()
    if args.fingerprint:
        if args.source or args.output or args.recipe != SOURCE / 'release/upstream.json':
            parser.error('--fingerprint uses the current source recipe only')
        print(fingerprint(SOURCE))
    else:
        if args.source is None or args.output is None:
            parser.error('build requires --source and --output')
        build(args.source, args.output, args.recipe)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit('[runtime] ' + str(error))
