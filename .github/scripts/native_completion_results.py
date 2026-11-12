"""Require each authoritative Native completion regression to execute and pass."""
from pathlib import Path
import sys

from tui_regression_results import verify_names

SUITES = {
    'core': (
        'native_completion_wire_preserves_original_call_session_and_actual_exit',
        'native_completion_unadmitted_rejection_and_unfinished_wait_are_unknown',
        'native_completion_header_does_not_promote_child_output_or_rewrite_public_status',
        'native_completion_initial_denial_excludes_started_backend_and_changed_hooks',
        'native_completion_retry_waits_preserve_attempt_identity_and_final_exit',
        'native_completion_completed_attempt_survives_declined_retry_without_successful_process',
        'native_completion_continuation_uses_its_own_call_and_original_process_session',
        'native_completion_dropped_dispatch_does_not_publish_late_wait',
        'native_completion_new_unknown_result_has_explicit_null_capability',
        'native_completion_timeout_keeps_public_124_until_original_os_wait_proves_exit',
        'native_completion_remote_termination_ack_is_not_local_wait_evidence',
        'native_completion_shutdown_keeps_admitted_persistence_until_flush_finishes',
        'native_completion_running_continuation_settles_only_its_current_invocation',
        'native_completion_hook_reload_before_admission_refuses_all_qualified_calls',
        'native_completion_admitted_hook_arc_drives_pre_post_and_permission_after_reload',
        'native_completion_advertised_scope_includes_actual_bash_hooks',
        'native_completion_admitted_snapshot_preserves_wait_after_later_hook_reload',
        'native_completion_network_permission_keeps_original_admitted_hooks',
        'native_completion_process_capability_is_per_tool_and_refuses_before_effects',
        'native_completion_local_driver_refusal_uses_actual_request_and_preserves_upstream',
        'native_completion_intercepted_patch_refuses_before_file_effects',
        'native_completion_successful_wait_without_numeric_exit_is_terminal',
        'native_completion_nonprocess_hook_refusal_preserves_discovery_and_plain_tools',
        'native_completion_code_mode_refuses_before_cell_dispatch',
        'native_completion_tree_wait_excludes_parent_only_and_earlier_attempt',
        'native_completion_pre_exec_refusal_preserves_started_attempt_scope',
        'native_completion_scope_header_preserves_root_exit_and_child_output',
        'native_completion_root_exit_retains_session_until_tree_stop',
        'native_completion_oneshot_keeps_original_supervised_attempt',
        'native_completion_pruning_stops_only_selected_scope',
        'native_completion_supervised_history_waits_for_tree_completion',
        'native_completion_provider_route_requires_versioned_native_ingress',
    ),
    'guardian': (
        'native_completion_guardian_route_uses_selected_provider_ingress',
    ),
    'protocol': (
        'native_completion_typescript_keeps_nullable_field_and_real_type_dependencies',
        'native_completion_terminal_wire_distinguishes_legacy_null_and_verified_wait',
    ),
    'rollout': (
        'native_completion_terminal_receipts_are_durable_in_both_history_modes',
    ),
    'app-server-protocol': (
        'native_completion_late_receipt_does_not_duplicate_or_rewrite_public_history',
    ),
    'hooks': (
        'native_completion_hooks_are_scoped_to_original_exec_or_continuation',
    ),
    'pty': (
        'native_completion_wait_errors_and_lossy_portable_signals_are_unconfirmed',
        'native_completion_driver_status_and_termination_request_are_not_os_wait',
        'native_completion_pipe_and_pty_preserve_actual_wait_codes',
        'native_completion_local_termination_confirms_only_after_wait',
        'native_completion_portable_pty_signal_proves_wait_without_numeric_exit',
        'native_completion_tree_guard_preserves_root_and_reaps_double_fork',
        'native_completion_tree_guard_stop_preserves_supervisor_and_other_jobs',
        'native_completion_tree_guard_root_kill_retains_detached_scope',
        'native_completion_tree_guard_rejects_setup_without_target_fallback',
        'native_completion_tree_guard_helper_loss_never_confirms_closure',
        'native_completion_tree_guard_preserves_pty_foreground_interrupt',
        'native_completion_tree_guard_hangup_stops_children_without_losing_owner',
        'native_completion_tree_guard_requires_frame_and_actual_helper_wait',
        'native_completion_tree_guard_closes_target_only_descriptors',
        'native_completion_guarded_backends_preserve_root_and_descendant_scope',
    ),
}


def verify(text, suite):
    return verify_names(text, SUITES[suite], 'Native completion ' + suite)


if __name__ == '__main__':
    suite, path = sys.argv[1:]
    count = verify(Path(path).read_text(), suite)
    print(f'{count} required Native completion {suite} regressions executed and passed')
