#!/usr/bin/env python3
"""Locate the maintained Termux adaptations in exact official Rust source.

This prepares a disposable build checkout. It never changes the installed runtime
or infers a release qualification from the fact that a source edit applied.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib


FULL_ACCESS = 'BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS'
WORKSPACE = 'BUILT_IN_PERMISSION_PROFILE_WORKSPACE'
DESCRIPTION = 'Run without a sandbox; the selected reviewer handles approval requests required by command rules and policy.'
REVIEW_DESCRIPTION = 'Auto-review approval requests without a sandbox.'


def preserve_pristine_times(source):
    """Avoid invalidating restored Cargo inputs merely because Git checked out."""
    def git(*args):
        return subprocess.check_output(['git', '-C', str(source), *args])
    epoch = int(git('show', '-s', '--format=%ct', 'HEAD'))
    changed = set(git('diff', '--name-only', '-z', 'HEAD').split(b'\0'))
    for path in git('ls-files', '-z', '--', 'codex-rs').split(b'\0'):
        if path and path not in changed:
            os.utime(source / os.fsdecode(path), (epoch, epoch), follow_symlinks=False)


def replace(text, old, new, where):
    if text.count(old) != 1:
        raise ValueError(f'{where}: expected one {old!r}, found {text.count(old)}')
    return text.replace(old, new, 1)


def function(text, name, transform, where):
    # rustfmt puts a function's closing brace at the declaration's indentation.
    # Resolve by declaration, never by release-dependent line/byte positions.
    pattern = rf'(?m)^( *)(?:pub(?:\([^\n]*?\))? )?(?:async )?fn {re.escape(name)}\b'
    matches = list(re.finditer(pattern, text))
    if len(matches) != 1:
        raise ValueError(f'{where}: expected one function {name}, found {len(matches)}')
    start = matches[0].start()
    end = re.search(r'(?m)^' + matches[0][1] + r'}$', text[matches[0].end():])
    if end is None:
        raise ValueError(f'{where}: cannot locate closing brace for {name}')
    stop = matches[0].end() + end.end()
    return text[:start] + transform(text[start:stop], f'{where}::{name}') + text[stop:]


def replacements(pairs):
    def apply(text, where):
        for old, new in pairs:
            text = replace(text, old, new, where)
        return text
    return apply


def plan(source):
    """Preflight every source expression before the caller writes any file."""
    changes = {}

    def edit(path, name, transform):
        path = source / 'codex-rs' / path
        if path.is_symlink() or not path.is_file():
            raise ValueError(f'{path}: expected regular official source')
        text = changes.get(path, path.read_text())
        changes[path] = function(text, name, transform, str(path.relative_to(source)))

    edit('uds/src/daemon_directory.rs', 'shared_daemon_socket_directory', replacements([
        ('fs::canonicalize("/tmp")?', 'fs::canonicalize("/proc/self/fd/35")?'),
        ('format!("codex-daemon-{uid}")', 'format!("{uid}")'),
    ]))
    edit('utils/approval-presets/src/lib.rs', 'builtin_approval_presets', replacements([
        (WORKSPACE, FULL_ACCESS),
        ('PermissionProfile::workspace_write()', 'PermissionProfile::Disabled'),
        ('Codex can read and edit files in the current workspace, and run commands. Approval is required to access the internet or edit other files. (Identical to Agent mode)', DESCRIPTION),
    ]))
    edit('tui/src/app.rs', 'auto_review_mode', replacements([
        (WORKSPACE, 'codex_protocol::models::' + FULL_ACCESS),
    ]))
    edit('tui/src/app_server_session.rs', 'thread_resume_params_from_config', replacements([
        ('thread_params_mode == ThreadParamsMode::Remote || !permission_overrides.profile',
         'thread_params_mode == ThreadParamsMode::Remote'),
    ]))
    edit('tui/src/chatwidget/permission_shortcuts.rs', 'handle_permission_shortcut', replacements([
        ('!matches!(preset.id, "read-only" | "auto")', 'preset.id != "auto"'),
    ]))

    def current_preset(text, where):
        old = '''"auto" => {
                let file_system_policy = current_permission_profile.file_system_sandbox_policy();
                matches!(
                    current_permission_profile,
                    PermissionProfile::Managed { .. }
                ) && file_system_policy.can_write_local_path_with_cwd(cwd, cwd)
                    && !file_system_policy.has_full_disk_write_access()
            }'''
        return replace(text, old,
                       '"auto" => matches!(current_permission_profile, PermissionProfile::Disabled),', where)

    edit('tui/src/chatwidget/permission_popups.rs', 'preset_matches_current', current_preset)
    edit('tui/src/chatwidget/permissions_menu.rs', 'try_select_permission_profile', replacements([
        ('":ask-for-approval" | ":approve-for-me" => ' + WORKSPACE,
         '":ask-for-approval" | ":approve-for-me" => ' + FULL_ACCESS),
    ]))
    edit('tui/src/chatwidget/permissions_menu.rs', 'permission_preset_description', replacements([
        ('Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace', DESCRIPTION),
    ]))

    def named_menu(text, where):
        binding = '''        let Some(read_only) = presets.iter().find(|preset| preset.id == "read-only") else {
            self.add_error_message(
                "Internal error: missing the 'read-only' approval preset.".to_string(),
            );
            return;
        };
'''
        item = '''        items.push(self.builtin_permission_mode_selection_item(
            &discovery,
            read_only,
            ":read-only",
            permission_preset_description(read_only).to_string(),
            AskForApproval::from(read_only.approval),
            ApprovalsReviewer::User,
        ));
'''
        text = replace(text, binding, '', where)
        text = replace(text, item, '', where)
        if text.count('":workspace"') != 2:
            raise ValueError(f'{where}: expected Ask and Auto workspace selections')
        return text.replace('":workspace"', FULL_ACCESS)

    edit('tui/src/chatwidget/permissions_menu.rs', 'open_permission_profiles_popup', named_menu)
    widget = source / 'codex-rs/tui/src/chatwidget.rs'
    changes[widget] = replace(widget.read_text(),
                              'const AUTO_REVIEW_DESCRIPTION: &str = "Only ask for actions detected as potentially unsafe";',
                              f'const AUTO_REVIEW_DESCRIPTION: &str = "{REVIEW_DESCRIPTION}";',
                              'tui/src/chatwidget.rs::AUTO_REVIEW_DESCRIPTION')
    # Remove imports whose only consumer was replaced. Other uses retain them.
    for path, text in changes.items():
        imported = f'use codex_protocol::models::{WORKSPACE};\n'
        if text.count(WORKSPACE) == 1 and imported in text:
            changes[path] = text.replace(imported, '', 1)
    return changes


def prepare(source, version, source_sha):
    if re.fullmatch(r'[0-9a-f]{40}', source_sha) is None:
        raise ValueError('expected exact official source commit')
    head = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
    if head != source_sha:
        raise ValueError('official source commit differs from release input')
    if subprocess.check_output(['git', '-C', str(source), 'status', '--porcelain']):
        raise ValueError('runtime source preparation requires a pristine disposable checkout')
    actual_version = tomllib.loads((source / 'codex-rs/Cargo.toml').read_text())['workspace']['package']['version']
    if actual_version != version or re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', version) is None:
        raise ValueError('official workspace version differs from release input')
    changes = plan(source)
    for path, text in changes.items():
        path.write_text(text)
    return {'version': version, 'source': source_sha,
            'changed_files': [str(path.relative_to(source)) for path in sorted(changes)]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('--version')
    parser.add_argument('--source-sha')
    parser.add_argument('--preserve-pristine-times', action='store_true')
    args = parser.parse_args()
    if args.preserve_pristine_times:
        if args.version or args.source_sha:
            parser.error('timestamp preservation does not accept release identity options')
        preserve_pristine_times(args.source.resolve())
    else:
        if not args.version or not args.source_sha:
            parser.error('source preparation requires --version and --source-sha')
        print(json.dumps(prepare(args.source.resolve(), args.version, args.source_sha), sort_keys=True))
