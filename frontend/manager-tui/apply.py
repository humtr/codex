"""Assemble owned Manager TUI sources and the bounded upstream integration patch."""
import argparse
import os
from pathlib import Path
import shutil
import stat
import subprocess
import tempfile

PACKAGE = Path(__file__).resolve().parent


def check_scope(upstream):
    """Check the assembled/format/lint input against the same owned boundary."""
    changed = subprocess.check_output(
        ['git', '-C', str(upstream), 'diff', '--no-renames', '--name-only', '-z', 'HEAD'])
    added = subprocess.check_output(
        ['git', '-C', str(upstream), 'ls-files', '--others', '--exclude-standard', '-z'])
    unexpected = sorted(path for path in set((changed + added).split(b'\0'))
                        if path and path not in (b'codex-rs/Cargo.lock',
                                                 b'codex-rs/core/src/config/mod.rs')
                        and not path.startswith(b'codex-rs/tui/'))
    if unexpected:
        raise ValueError('unexpected upstream frontend changes: ' +
                         ', '.join(path.decode(errors='backslashreplace') for path in unexpected))


def apply(upstream, package=PACKAGE):
    upstream = upstream.resolve(strict=True)
    source = package / 'src'
    target = upstream / 'codex-rs/tui/src'
    if source.is_symlink() or not source.is_dir():
        raise ValueError('owned frontend source root must be a real directory')
    if not target.is_dir():
        raise ValueError('upstream TUI source is missing')
    payloads = []
    for path in sorted(source.rglob('*')):
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode):
            continue
        if not stat.S_ISREG(mode) or path.suffix not in {'.rs', '.snap', '.json'}:
            raise ValueError('unexpected owned frontend input: ' + str(path))
        destination = target / path.relative_to(source)
        if destination.exists() or destination.is_symlink():
            raise ValueError('owned frontend source collides with upstream: ' + str(destination))
        for parent in destination.parents:
            if parent == upstream:
                break
            if parent.is_symlink() or (parent.exists() and not parent.is_dir()):
                raise ValueError('unsafe upstream source parent: ' + str(parent))
        payloads.append((path, destination))
    if not payloads:
        raise ValueError('owned frontend source set is empty')
    patch = package / 'native.patch'
    # All collisions and patch applicability are checked before any mutation.
    command = ['git', '-C', str(upstream), 'apply']
    direct = subprocess.run(command + ['--check', str(patch.resolve())], capture_output=True)
    if direct.returncode == 0:
        subprocess.run(command + [str(patch.resolve())], check=True)
    else:
        # Locate changed context using Git's original blobs. A scratch index
        # keeps a merge conflict from changing any upstream file or real index.
        with tempfile.TemporaryDirectory(prefix='frontend-rebase-') as temporary:
            environment = {**os.environ, 'GIT_INDEX_FILE': str(Path(temporary) / 'index')}
            subprocess.run(['git', '-C', str(upstream), 'read-tree', 'HEAD'], env=environment, check=True)
            merged = subprocess.run(command + ['--cached', '--3way', str(patch.resolve())],
                                    env=environment, capture_output=True)
            if merged.returncode:
                raise ValueError('frontend source merge failed before mutation; fetch upstream-base objects and inspect conflicts: '
                                 + merged.stderr.decode(errors='replace'))
            rebased = subprocess.check_output(['git', '-C', str(upstream), 'diff', '--cached',
                                               '--binary', 'HEAD'], env=environment)
            subprocess.run(command + ['--check', '-'], input=rebased, check=True)
            subprocess.run(command + ['-'], input=rebased, check=True)
    for source, destination in payloads:
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as output, source.open('rb') as input_file:
            shutil.copyfileobj(input_file, output)
    return len(payloads)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('upstream', type=Path)
    parser.add_argument('--check-scope', action='store_true',
                        help='validate assembled/format/lint scope without changing source')
    options = parser.parse_args()
    if options.check_scope:
        check_scope(options.upstream)
    else:
        print(f'assembled {apply(options.upstream)} owned frontend sources')
