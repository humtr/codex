"""Check first-update and no-op output against original signed/public inputs."""
import argparse
from datetime import datetime, timedelta, timezone
import json
from pathlib import Path

from rald5_publication import parse_manifest


def variants(current, target, component, publication, base_publication):
    old, old_files = parse_manifest(current)
    new, files = parse_manifest(target)
    edition = 'Full' if any(name == 'manager' for name, _, _ in files) else 'Base'
    previous = 'Full' if any(name == 'manager' for name, _, _ in old_files) else 'Base'
    prefix = f'Termux release: {previous} #{old["release_sequence"]}'
    if old['generation_id'] != new['generation_id']:
        prefix += f' -> {edition} #{new["release_sequence"]}'
    display = json.loads(component.read_text())['display']
    if edition == 'Full':
        prefix += f' · Base #{display["base_sequence"]}'
    legacy = prefix + f' · Release date: {display["release_date"]} (UTC)'
    def original(path, tag):
        if path is None:
            return None
        info = json.loads(path.read_text())
        assert info['tag_name'] == tag and info['draft'] is False
        return info

    def stamp(info):
        if info is None:
            return 'unavailable'
        value = datetime.strptime(info['published_at'], '%Y-%m-%dT%H:%M:%SZ').replace(tzinfo=timezone.utc)
        return value.astimezone(timezone(timedelta(hours=9))).strftime('%Y-%m-%d %H:%M:%S KST (UTC+09:00)')

    def notes(info, section, url):
        entries = []
        name = None
        for line in (info or {}).get('body', '').splitlines():
            if line.startswith('## '):
                name = line[3:].strip()
            elif line.startswith('- ') and name == section:
                text = line[2:].strip()
                entries.append(text[:180] + ('…' if len(text) > 180 else ''))
        entries = entries[:3]
        output = [section + ' changes:' + ('' if entries else ' unavailable')]
        if entries:
            output += ['  - ' + entry for entry in entries] + ['  Full notes: ' + url]
        return output

    package = original(publication, new['generation_id'])
    base = original(base_publication, display.get('base_generation'))
    variants = []
    for info in [None, package] if package else [None]:
        for base_info in [None, base] if base else [None]:
            core = base_info if display.get('base_generation') else info
            if info is None and core is None:
                variants.append([legacy])
                continue
            full = [prefix + ' · Published: ' + stamp(info)] if info else [legacy]
            url = 'https://github.com/humtr/codex/releases/tag/' + new['generation_id']
            if edition == 'Full':
                component = 'Base #' + str(display['base_sequence']) if display.get('base_generation') else 'Full #' + new['release_sequence'] + ' package'
                full.append(f'Core ({component}) · Published: ' + stamp(core))
            core_url = 'https://github.com/humtr/codex/releases/tag/' + display.get('base_generation', new['generation_id']) if display.get('base_generation') else url
            full += notes(core, 'Core', core_url)
            for section, file, label in [('Manager', 'manager', 'Manager'), ('TUI', 'helpers/2', 'Enhanced TUI')]:
                if any(name == file for name, _, _ in files):
                    full.append(f'{label} ({edition} #{new["release_sequence"]} package) · Published: ' + stamp(info))
                    full += notes(info, section, url)
            variants.append(full)
    return variants


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['actual', 'plain', 'component', 'current', 'target']:
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--publication', type=Path)
    parser.add_argument('--base-publication', type=Path)
    parser.add_argument('--allow-legacy', action='store_true')
    args = parser.parse_args()
    plain = args.plain.read_text().splitlines()
    expected = [[plain[0], *detail, *plain[1:]] for detail in
                variants(args.current, args.target, args.component, args.publication, args.base_publication)]
    if args.allow_legacy:
        expected.append(plain)
    actual = args.actual.read_text().splitlines()
    assert actual in expected, (actual, expected)
