"""Allow only the exact Termux legacy-policy configuration compatibility guards."""
import argparse
from pathlib import Path
import subprocess

GUARDS = {
    'codex-rs/core/src/config/mod.rs': ('cfg', 8),
    'codex-rs/app-server/src/config_manager_service.rs': ('config', 4),
}


TRANSFORMS = {'codex-rs/ext/guardian-reviewer/src/routing.rs': [('pub fn routes_approval_policy_to_guardian(\n'
                                                    '    policy: AskForApproval,\n'
                                                    '    reviewer: ApprovalsReviewer,\n'
                                                    ') -> bool {\n'
                                                    '    matches!(\n'
                                                    '        policy,\n'
                                                    '        AskForApproval::OnRequest | '
                                                    'AskForApproval::Granular(_)\n'
                                                    '    ) && reviewer == '
                                                    'ApprovalsReviewer::AutoReview\n'
                                                    '}\n',
                                                    'fn termux_untrusted_review() -> bool {\n'
                                                    '    std::env::var_os("CODEX_PROFILE_CORE")\n'
                                                    '        .as_deref()\n'
                                                    '        .is_some_and(|core| '
                                                    'std::path::Path::new(core).is_absolute())\n'
                                                    '}\n'
                                                    '\n'
                                                    'pub fn routes_approval_policy_to_guardian(\n'
                                                    '    policy: AskForApproval,\n'
                                                    '    reviewer: ApprovalsReviewer,\n'
                                                    ') -> bool {\n'
                                                    '    (matches!(\n'
                                                    '        policy,\n'
                                                    '        AskForApproval::OnRequest | '
                                                    'AskForApproval::Granular(_)\n'
                                                    '    ) || matches!(policy, '
                                                    'AskForApproval::UnlessTrusted) && '
                                                    'termux_untrusted_review())\n'
                                                    '        && reviewer == '
                                                    'ApprovalsReviewer::AutoReview\n'
                                                    '}\n')],
 'codex-rs/ext/guardian-reviewer/src/routing_tests.rs': [('(AskForApproval::UnlessTrusted, [false, '
                                                          'false]),',
                                                          '(\n'
                                                          '            '
                                                          'AskForApproval::UnlessTrusted,\n'
                                                          '            [false, '
                                                          'super::termux_untrusted_review()],\n'
                                                          '        ),')],
 'codex-rs/ext/guardian-v2/src/sync_reviewer/mod.rs': [('matches!(\n'
                                                        '                    '
                                                        'input.config.permissions.approval_policy.value(),\n'
                                                        '                    '
                                                        'AskForApproval::OnRequest | '
                                                        'AskForApproval::Granular(_)\n'
                                                        '                )',
                                                        'codex_guardian_reviewer::routes_approval_policy_to_guardian(\n'
                                                        '                    '
                                                        'input.config.permissions.approval_policy.value(),\n'
                                                        '                    '
                                                        'input.config.approvals_reviewer,\n'
                                                        '                )'),
                                                       ('use '
                                                        'codex_protocol::protocol::AskForApproval;\n',
                                                        '')],
 'codex-rs/ext/guardian-v2/src/async_scorer/approval.rs': [('matches!(\n'
                                                            '                    '
                                                            'input.approval_policy,\n'
                                                            '                    '
                                                            'AskForApproval::OnRequest | '
                                                            'AskForApproval::Granular(_)\n'
                                                            '                )',
                                                            'codex_guardian_reviewer::routes_approval_policy_to_guardian(\n'
                                                            '                    '
                                                            'input.approval_policy,\n'
                                                            '                    '
                                                            'input.approvals_reviewer,\n'
                                                            '                )'),
                                                           ('use '
                                                            'codex_protocol::protocol::AskForApproval;\n',
                                                            '')]}


def compatible(source, variable, indent):
    pad = ' ' * indent
    before = pad + f'if {variable}.approval_policy == Some(AskForApproval::UnlessTrusted) {{'
    after = (pad + f'if {variable}.approval_policy == Some(AskForApproval::UnlessTrusted)\n'
             + pad + '    && !std::env::var_os("CODEX_PROFILE_CORE")\n'
             + pad + '        .as_deref()\n'
             + pad + '        .is_some_and(|core| std::path::Path::new(core).is_absolute())\n'
             + pad + '{')
    if source.count(before) != 1:
        raise ValueError('official configuration guard changed; fresh qualification required')
    return source.replace(before, after)


def verify(root):
    changed = subprocess.check_output(['git', '-C', str(root), 'diff', '--name-only', 'HEAD', '-z']).split(b'\0')
    untracked = subprocess.check_output(['git', '-C', str(root), 'ls-files', '--others', '--exclude-standard', '-z']).split(b'\0')
    for raw in changed + untracked:
        path = raw.decode()
        if path and path != 'codex-rs/Cargo.lock' and not path.startswith('codex-rs/tui/') and path not in GUARDS and path not in TRANSFORMS:
            raise ValueError('unowned backend source change: ' + path)
    for path, (variable, indent) in GUARDS.items():
        original = subprocess.check_output(['git', '-C', str(root), 'show', 'HEAD:' + path]).decode()
        if (root / path).read_text() != compatible(original, variable, indent):
            raise ValueError('backend compatibility guard differs: ' + path)

    for path, replacements in TRANSFORMS.items():
        original = subprocess.check_output(['git', '-C', str(root), 'show', 'HEAD:' + path]).decode()
        for before, after in replacements:
            if original.count(before) != 1:
                raise ValueError('official reviewer source changed: ' + path)
            original = original.replace(before, after)
        if (root / path).read_text() != original:
            raise ValueError('backend review routing differs: ' + path)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('upstream', type=Path)
    verify(parser.parse_args().upstream)
    print('Only exact Core-bound configuration and native Untrusted review routing changed.')
