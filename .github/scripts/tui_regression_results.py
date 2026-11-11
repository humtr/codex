"""Require executed positive Provider/catalog/switch regressions, not compilation."""
from pathlib import Path
import re
import sys

REQUIRED = (
    'termux_chat_models_current_provider_catalog_preserves_openai_order_without_app_lookup',
    'termux_chat_models_cold_openai_return_uses_upstream_catalog_and_rejects_foreign_cache',
    'termux_chat_models_live_catalog_requires_current_account_and_exact_private_snapshot',
    'termux_chat_models_inline_choices_use_the_same_installed_catalog_as_the_picker',
    'termux_chat_models_same_provider_uses_only_changed_session_actions_without_persistence',
    'termux_provider_slash_uses_owned_model_catalog_and_does_not_guess_names',
    'bound_chat_synchronization_orders_native_input_and_retains_cancelled_or_failed_drafts',
    'active_chat_provider_steer_uses_native_input_without_synchronization',
    'replayed_rate_limit_error_preserves_current_account_and_initial_prompt',
    'enqueue_primary_thread_session_submits_checkpoint_after_replayed_quota_error',
)


def verify(text):
    text = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', text)
    lines = text.splitlines()
    for name in REQUIRED:
        results = [line for line in lines if re.search(r'\b' + re.escape(name) + r'(?:\s|$)', line)]
        passed = [line for line in results if re.match(r'^\s*PASS\b', line)]
        rejected = [line for line in results if re.match(r'^\s*(?:FAIL|SKIP|TIMEOUT|LEAK)\b', line)]
        if len(passed) != 1 or rejected:
            raise ValueError('required TUI regression did not pass exactly once: ' + name)
    return len(REQUIRED)


if __name__ == '__main__':
    count = verify(Path(sys.argv[1]).read_text())
    print(f'{count} required Provider/catalog/switch regressions executed and passed')
