#!/usr/bin/env python3
"""Build the optional Termux tunnel from official source, without another checkout.

Release preparation and explicit source-build recovery use this same recipe.
Normal installation consumes signed prebuilt packages; this command never
selects/restarts a service, edits credentials or publishes an artifact.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

RECIPE_ROOT = Path(__file__).resolve().parents[1] / 'transport/tunnel-client'
LATEST = 'https://api.github.com/repos/openai/tunnel-client/releases/latest'


def run(argv, *, cwd=None, env=None, capture=False):
    return subprocess.run(list(map(str, argv)), cwd=cwd, env=env, check=True,
                          stdout=subprocess.PIPE if capture else None,
                          text=capture).stdout


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def select_release(requested, recipe, fetch):
    version = requested
    if requested == 'latest':
        release = fetch()
        if release.get('draft') or release.get('prerelease'):
            raise ValueError('official stable release metadata is not stable')
        version = release.get('tag_name')
    if version != recipe['version']:
        raise ValueError(f'upstream {version!r} is outside the validated recipe '
                         f'{recipe["version"]}; update the wrapper recipe; '
                         'no older build was selected')
    return version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', default='latest',
                        help='latest official stable or exact validated version')
    parser.add_argument('--output', required=True, type=Path,
                        help='absent absolute artifact directory; no service activation')
    args = parser.parse_args()
    if not args.output.is_absolute() or args.output.exists():
        parser.error('--output must name an absent absolute directory')
    recipe = json.loads((RECIPE_ROOT / 'recipe.json').read_text())
    patch = RECIPE_ROOT / 'termux.patch'
    if platform.system() not in ('Linux', 'Android') or platform.machine() not in ('aarch64', 'arm64'):
        parser.error('this recipe requires an Android/arm64 Termux toolchain')
    for tool in ('git', 'curl', 'go', 'clang'):
        if shutil.which(tool) is None:
            parser.error('missing build tool '+tool+
                         '; install Python, git, curl, golang and clang with the '
                         'Termux package manager for this explicit build')
    go_target = json.loads(run(['go', 'env', '-json', 'GOOS', 'GOARCH'], capture=True))
    if go_target != {'GOOS': 'android', 'GOARCH': 'arm64'}:
        parser.error('an actual Android/arm64 Go toolchain is required')
    version = select_release(args.version, recipe, lambda: json.loads(run([
        'curl', '--fail', '--silent', '--show-error', '--location',
        '--proto', '=https', '--proto-redir', '=https', '--connect-timeout', '15',
        '--max-time', '60', '--max-filesize', '1048576', LATEST], capture=True)))
    # Keep original logs/source on failure. Owned successful checkout is temporary;
    # output contains exactly the consumer artifact and build declaration.
    work = Path(tempfile.mkdtemp(prefix='codex-tunnel-build-', dir=os.environ['TMPDIR']))
    print('Tunnel build workspace: '+str(work), flush=True)
    source = work / 'source'
    source.mkdir()
    cache = Path(os.environ.get('XDG_CACHE_HOME', str(Path.home()/'.cache'))) / 'codex'
    home = work / 'home'
    home.mkdir(mode=0o700)
    module_cache = run(['go', 'env', 'GOMODCACHE'], capture=True).strip()
    env = {key: os.environ[key] for key in
           ('PATH', 'PREFIX', 'TMPDIR', 'LANG', 'SSL_CERT_FILE') if key in os.environ}
    env.update(recipe['target'], HOME=str(home), CC=shutil.which('clang'),
               GOCACHE=str(cache/'tunnel-go-build'), GOMODCACHE=module_cache,
               GOMAXPROCS='2', GOFLAGS='-p=1', GOTOOLCHAIN='local',
               GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null',
               GIT_TERMINAL_PROMPT='0')
    # Independent of user clones/remotes, Git aliases, profiles and credentials.
    run(['git', 'init', '-q', source], env=env)
    run(['git', '-c', 'protocol.file.allow=never', 'fetch', '--no-tags', '--depth=1',
         recipe['upstream_repository'], recipe['commit']], cwd=source, env=env)
    actual = run(['git', 'rev-parse', 'FETCH_HEAD'], cwd=source, env=env, capture=True).strip()
    if actual != recipe['commit']:
        raise ValueError('official source commit differs from the validated recipe')
    run(['git', 'checkout', '-q', '--detach', actual], cwd=source, env=env)
    if (source/'pkg/version/VERSION').read_text().strip() != version[1:]:
        raise ValueError('official source version differs from selected release')
    run(['git', 'apply', '--check', '--whitespace=error', patch], cwd=source, env=env)
    run(['git', 'apply', '--whitespace=error', patch], cwd=source, env=env)
    run(['go', 'test', './pkg/controlplane/internal', '-count=1', '-run',
         recipe['regression_test']], cwd=source, env=env)
    run(['go', 'test', *recipe['test_packages']], cwd=source, env=env)
    binary = work / 'tunnel-client'
    identity = 'github.com/openai/tunnel-client/pkg/version.'
    # The upstream commit and patch identity remain separately labelled.
    recipe_sha, patch_sha = sha(RECIPE_ROOT / 'recipe.json'), sha(patch)
    derived_version = version[1:]+'-termux.'+patch_sha[:12]
    flags = '-s -w -X '+identity+'GitSHA='+actual+' -X '+identity+'semanticVersion='+derived_version
    run(['go', 'build', '-trimpath', '-buildvcs=false', '-ldflags='+flags,
         '-o', binary, './cmd/client'], cwd=source, env=env)
    observed = run([binary, '--version'], capture=True).strip()
    if not observed.startswith(derived_version+'+') or actual not in observed:
        raise ValueError('built executable did not report upstream and patch identity')
    metadata = run(['go', 'version', '-m', binary], capture=True)
    for key, value in recipe['target'].items():
        if '\t'+key+'='+value not in metadata:
            raise ValueError('built executable target mismatch: '+key)
    output = args.output
    output.mkdir(mode=0o700)
    try:
        shutil.copy2(binary, output / 'tunnel-client')
        (output / 'tunnel-client').chmod(0o755)
        shutil.copy2(source / 'LICENSE', output / 'LICENSE')
        declaration = dict(upstream_version=version, upstream_commit=actual,
                           recipe_sha256=recipe_sha, patch_sha256=patch_sha,
                           binary_sha256=sha(binary),
                           target=recipe['target'], go=run(['go', 'version'], capture=True).strip(),
                           version_output=observed, tests=recipe['test_packages'],
                           activation_performed=False)
        (output / 'build.json').write_text(json.dumps(declaration, indent=2)+'\n')
    except BaseException:
        # Preserve partial artifacts for diagnosis; never present them as installed.
        print('Incomplete build output retained at '+str(output), flush=True)
        raise
    shutil.rmtree(work)
    print(json.dumps(dict(accepted=True, output=str(output), **declaration)), flush=True)


if __name__ == '__main__':
    os.umask(0o077)
    main()
