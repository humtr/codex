"""Assemble owned Manager TUI sources and the bounded upstream integration patch."""
import argparse
from pathlib import Path
import shutil
import stat
import subprocess

PACKAGE = Path(__file__).resolve().parent


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
    subprocess.run(['git', '-C', str(upstream), 'apply', '--check', str(patch.resolve())], check=True)
    subprocess.run(['git', '-C', str(upstream), 'apply', str(patch.resolve())], check=True)
    for source, destination in payloads:
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as output, source.open('rb') as input_file:
            shutil.copyfileobj(input_file, output)
    return len(payloads)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('upstream', type=Path)
    options = parser.parse_args()
    print(f'assembled {apply(options.upstream)} owned frontend sources')
