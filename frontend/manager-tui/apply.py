"""Assemble owned Manager TUI sources and the bounded upstream integration patch."""
import argparse
import json
from pathlib import Path
import shutil
import stat
import subprocess

PACKAGE = Path(__file__).resolve().parent


def preparation_files(upstream, path):
    value = json.loads(path.read_text())
    revision = subprocess.check_output(['git', '-C', str(upstream), 'rev-parse', 'HEAD'], text=True).strip()
    files = value.get('changed_files')
    if value.get('source') != revision or not isinstance(files, list) or not files:
        raise ValueError('Native preparation does not describe this upstream source')
    if any(not isinstance(name, str) or not name.startswith('codex-rs/')
           or '..' in Path(name).parts for name in files):
        raise ValueError('invalid Native preparation source path')
    return {name.encode() for name in files}


def check_scope(upstream, native_files=()):
    """Check the assembled/format/lint input against the same owned boundary."""
    changed = subprocess.check_output(
        ['git', '-C', str(upstream), 'diff', '--no-renames', '--name-only', '-z', 'HEAD'])
    added = subprocess.check_output(
        ['git', '-C', str(upstream), 'ls-files', '--others', '--exclude-standard', '-z'])
    unexpected = sorted(path for path in set((changed + added).split(b'\0'))
                        if path and path != b'codex-rs/Cargo.lock'
                        and path not in native_files
                        and not path.startswith(b'codex-rs/app-server-protocol/schema/')
                        and not path.startswith(b'codex-rs/tui/'))
    if unexpected:
        raise ValueError('unexpected upstream frontend changes: ' +
                         ', '.join(path.decode(errors='backslashreplace') for path in unexpected))


def apply(upstream, package, builder):
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
    # The common Rust producer owns patch reconciliation for both Native and TUI.
    subprocess.run([str(builder), 'apply-source-patch', '--source', str(upstream),
                    '--patch', str((package/'native.patch').resolve())], check=True)
    for source, destination in payloads:
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as output, source.open('rb') as input_file:
            shutil.copyfileobj(input_file, output)
    return len(payloads)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('upstream', type=Path)
    parser.add_argument('--builder', type=Path)
    parser.add_argument('--native-preparation', type=Path, required=True,
                        help='existing shared Native source-preparation result')
    parser.add_argument('--check-scope', action='store_true',
                        help='validate assembled/format/lint scope without changing source')
    options = parser.parse_args()
    native_files = preparation_files(options.upstream, options.native_preparation)
    check_scope(options.upstream, native_files)
    if not options.check_scope:
        if options.builder is None:
            parser.error('--builder is required for source assembly')
        print(f'assembled {apply(options.upstream, PACKAGE, options.builder)} owned frontend sources')
