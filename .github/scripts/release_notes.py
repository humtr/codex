"""Publish component-scoped human notes from the exact accepted source."""
import argparse
from pathlib import Path

SECTIONS = ('Core', 'Manager', 'TUI', 'Provider')
NOTE_BYTE_LIMIT = 16 * 1024


def render(source, component):
    wanted = {'base': ('Core',), 'full': ('Core', 'Manager', 'TUI'),
              'provider': ('Provider',)}[component]
    path = source / 'release/notes.md'
    if path.is_symlink() or not path.is_file():
        raise ValueError('release notes must be a bounded regular source file')
    with path.open('rb') as stream:
        data = stream.read(NOTE_BYTE_LIMIT + 1)
    if len(data) > NOTE_BYTE_LIMIT:
        raise ValueError('release notes exceed 16 KiB')
    sections = {}
    current = None
    for line in data.decode('utf-8').splitlines():
        if line.startswith('## '):
            current = line[3:].strip()
            if current not in SECTIONS or current in sections:
                raise ValueError('unknown or duplicate component release notes')
            sections[current] = []
        elif line.strip():
            if current is None or not line.startswith('- ') or any(ord(c) < 32 for c in line):
                raise ValueError('release notes require component headings and plain bullet entries')
            sections[current].append(line)
    if any(not sections.get(name) for name in wanted):
        raise ValueError('release notes are missing a packaged component')
    return '\n\n'.join('## '+name+'\n'+'\n'.join(sections[name]) for name in wanted)+'\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--component', choices=['base', 'full', 'provider'], required=True)
    args = parser.parse_args()
    print(render(args.source, args.component), end='')
