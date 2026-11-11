#!/usr/bin/env python3
"""CI input identities for qualified runtime reuse; compilation is Rust-owned."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import re
import tomllib

SOURCE = Path(__file__).resolve().parents[2]

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
                  lock_audit=value['qualification']['lock_audit'], files={name: hashlib.sha256(read(name)).hexdigest()
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


# These steps only select, verify or carry an already-built raw artifact. All
# unknown steps remain byte-bound: adding a compiler command never disappears
# merely because it was not in a historical producer allowlist.
RAW_CONSUMER_STEPS = frozenset({
    'Reuse an admitted runtime when only wrapper components changed',
    'Retain runtime subset from signed Base',
    'Resolve exact-source qualified runtime artifact',
    'Resolve production-equivalent raw runtime artifact',
    'Download the qualified runtime across workflow refs',
    'Download original raw runtime production',
    'Resolve portable runtime recipe',
    'Restore completed raw build for identical production inputs',
    'Verify reusable raw runtime before qualification',
    'Record original raw runtime production identity',
    'Cache completed raw build without granting admission',
    'Package runtime and preparation evidence with original modes',
    'Retain runtime and preparation evidence',
    'Retain original raw runtime production receipt',
    'Retain reusable compile cache',
})
RAW_COMPILER_STEPS = frozenset({
    'Fetch exact wrapper recipe', 'Provision exact official build toolchain',
    'Build shared Rust source preparation', 'Fetch exact official source',
    'Install official musl dependency tooling',
    'Build and verify through the shared runtime producer',
})


# These are audited, effect-isolated transfer/control step bodies from the v1
# and v2 workflows. A familiar name is never authority to discard new commands,
# environment settings or working-directory/defaults. Unknown bodies are hashed.
# This is NOT an artifact/cache fingerprint allowlist: both complete production
# input objects, immutable producer provenance and current bytes must still match.
RAW_CONSUMER_STEP_BODIES = frozenset({
    '1d9a3f1c4bd11e4855c0912e6dfc2e5c34871958e305a83116b9398729882529',  # Package runtime and preparation evidence with original modes
    '24011ec579fa49dc2dfd8c5268d3fe8ced29223251b296a68084734b99ab4e61',  # Retain runtime and preparation evidence
    '2fdc074c852546fcd6da7b2063ad2046ec9fc3523f92ae46ff34e4f1d2e00073',  # Verify reusable raw runtime before qualification
    '3e858bf44b1dd175280f06ba7be86cc7f406fcbc0445d701c1444ae1ea7d07b7',  # Cache completed raw build without granting admission
    '4f456071b390a3b2f4476a21ddbb8394e9ee635b3521ba37044d2ee2d1578fbf',  # Resolve portable runtime recipe
    '67b135747ea6934679438799c562feb1e57c93970062c22f903b4a3034138e9a',  # Retain original raw runtime production receipt
    '8101aae3cdb0fffd0ae70d7aefad4196d64e1e855efc5310ffe76aeff3647cbb',  # Restore completed raw build for identical production inputs
    '9135ad7942701ed57355fabe92380d4803569f2938a8e30f8bee0a8ecce14676',  # Retain reusable compile cache
    '9b58b7573396838d5dee056973073e318f2c3c0f986789114d009e2b2c0af1b3',  # Resolve exact-source qualified runtime artifact
    'a53df5401abac0529c2f5c641f31783f1eda09e849e19ba1315f13ecbc8e2066',  # Retain runtime and preparation evidence
    'b2e5e10de17fdce44d5577cdf4ed7fc0d758dfb49233ef55fcfcbd6b6bd24872',  # Download the qualified runtime across workflow refs
    'c1e673170a55293af1e458b25cfe985744e49b5156a1ee1f953e393cad7b5354',  # Record original raw runtime production identity
    'e02f51f5c094fda30d1bb8daf011e85e0c092792635d0699bdfba9821fc5840d',  # Verify reusable raw runtime before qualification
    'e8825a0dc93e3a9d04058eec36dcae2a4dfbe2f2a2cf643cb555d7f2e3e94348',  # Resolve production-equivalent raw runtime artifact
    'eff05aee54e2de95c5b504fba1ae83d267ecc8c199c917a9cc53f882a21ba835',  # Resolve portable runtime recipe
    'f6de0aa6ace425656d3c555b237689c1d2dd6aa98ba7b122901f0c63a0293989',  # Retain runtime subset from signed Base
    'fb8e22be7a44255b72fa758e7f2ec757b6355a36862e3e935c6567e768ccf58e',  # Download original raw runtime production
    'ff02015ea4ef0da5ae6b618ad33384761ae932f0f3397bf4abcd5c85a80df6d7',  # Reuse an admitted runtime when only wrapper components changed
})


def compiler_workflow(contents):
    """Bind compiler settings and every non-consumer step, without YAML rewrites."""
    header, marker, body = contents.partition('    steps:\n')
    if not marker:
        raise ValueError('raw runtime workflow has no compiler steps')
    matches=list(re.finditer(r'(?m)^      - name: ([^\n]+)\n',body))
    if (not matches or body[:matches[0].start()].strip()
            or len(matches)!=len(re.findall(r'(?m)^      - ',body))):
        raise ValueError('raw runtime workflow contains an unnamed producer step')
    names=[match[1] for match in matches]
    if len(names)!=len(set(names)) or not RAW_COMPILER_STEPS.issubset(names):
        raise ValueError('raw runtime compiler step identity differs')
    kept=[]
    for index,match in enumerate(matches):
        block=body[match.start():matches[index+1].start() if index+1<len(matches) else len(body)]
        body_digest=hashlib.sha256(block.rstrip().encode()).hexdigest()
        if match[1] not in RAW_CONSUMER_STEPS or body_digest not in RAW_CONSUMER_STEP_BODIES:
            kept.append(block)
    return header+marker+''.join(kept)


def production_inputs(source, revision='HEAD'):
    """Actual raw compiler inputs, separate from transport and admission recipes.

    The current implementation evaluates both immutable source trees. It never
    runs the old tree's Python and never equates unrelated legacy cache keys.
    The full historical fingerprint remains unchanged for installed metadata.
    """
    tree=subprocess.check_output(['git','-C',str(source),'ls-tree','-r',revision],
                                 stderr=subprocess.DEVNULL).decode().splitlines()
    modes={row.split('\t',1)[1]:row.split('\t',1)[0].split()[:2] for row in tree}
    def read(name):
        if modes.get(name) not in (['100644','blob'],['100755','blob']):
            raise ValueError('raw compiler input is not a regular Git file: '+name)
        return subprocess.check_output(['git','-C',str(source),'show',revision+':'+name],
                                       stderr=subprocess.DEVNULL)
    value=json.loads(read('release/upstream.json'))
    inventory=assets(read(ASSET_MANIFEST).decode())
    excluded={ASSET_MANIFEST,'.github/scripts/runtime_inputs.py','.github/scripts/runtime_artifact.py'}
    workflow='.github/workflows/termux-runtime-source.yml'
    if not excluded.issubset(inventory) or workflow not in inventory:
        raise ValueError('unknown raw compiler input inventory')
    files={}
    for name,role in inventory.items():
        if name in excluded or name=='release/upstream.json':
            continue
        contents=read(name)
        if name==workflow:
            contents=compiler_workflow(contents.decode()).encode()
        files[name]={'role':role,'mode':modes[name][0],'sha256':hashlib.sha256(contents).hexdigest()}
    configs={}
    for name in sorted(modes):
        if not name.startswith('.cargo/'):
            continue
        contents=read(name)
        if name in {'.cargo/config','.cargo/config.toml'}:
            config=tomllib.loads(contents.decode())
            environment=config.get('env',{})
            threads=environment.get('RUST_TEST_THREADS')
            if isinstance(threads,str) and threads.isdecimal():
                environment.pop('RUST_TEST_THREADS')
                if not environment:config.pop('env',None)
            if not config:continue
            contents=canonical(config)
        configs[name]={'mode':modes[name][0],'sha256':hashlib.sha256(contents).hexdigest()}
    return {'schema':'codex-raw-runtime-build-inputs-v2',
            'upstream':value['upstream'],'rust':value['toolchains']['frontend_rust'],
            'lock_audit':value['qualification']['lock_audit'],
            'workspace':tomllib.loads(read('Cargo.toml').decode()),'cargo_config':configs,
            'producer_dependencies':locked_packages(cargo_dependencies(read,'crates/release-builder/Cargo.toml')),
            'files':files}


def production_fingerprint(source, revision='HEAD'):
    return hashlib.sha256(canonical(production_inputs(source,revision))).hexdigest()


def adapted_production_fingerprint(source,revision='HEAD'):
    """Module rN inputs: compiler plus actual adaptation, not qualification code."""
    value=production_inputs(source,revision)
    def read(name):
        return subprocess.check_output(['git','-C',str(source),'show',revision+':'+name],stderr=subprocess.DEVNULL)
    tree=subprocess.check_output(['git','-C',str(source),'ls-tree','-r',revision,'--',
                                 'crates/release-builder','crates/runtime-binding'])
    value['adaptation']={'core_rust':json.loads(read('release/upstream.json'))['toolchains']['core_rust'],
                        'sources':tree.decode().splitlines(),
                        'binding_dependencies':locked_packages(cargo_dependencies(read,'crates/runtime-binding/Cargo.toml'))}
    return hashlib.sha256(canonical(value)).hexdigest()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--production',action='store_true')
    args=parser.parse_args()
    print(production_fingerprint(SOURCE) if args.production else fingerprint(SOURCE))
