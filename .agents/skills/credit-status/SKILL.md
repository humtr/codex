---
name: credit-status
description: Use when checking OpenAI docs, codex-rs source, or account surfaces for credit status and reset-credit metadata, and when deciding whether that data is public, internal, or unsupported.
---

# Credit Status

## Overview

This is an execution skill. When invoked, fetch the current account's reset-credit inventory immediately and show the full upstream metadata.

Default source order:

1. Current account fetch through the internal `codex-rs` path.
2. Official OpenAI docs only if the internal fetch fails.

## Workflow

1. Run `scripts/fetch_reset_credits.py`.
2. Render the result as a plain-text block, not JSON.
3. Show the account-level totals first.
4. Show every credit record in `granted_at` order.
5. Preserve all upstream fields that arrive from the endpoint, including top-level fields beyond the totals.
6. If the fetch fails, report the exact failure and stop.

## What To Return

For supported data, render the full live payload in this style:

```text
조회 결과:
  - available_count: 3
  - total_earned_count: 0

  1. RateLimitResetCredit_...
      - status: available
      - granted_at: 2026-06-12T02:12:33.195949Z
      - expires_at: 2026-07-12T02:12:33.195949Z
      - profile_user_id: Codex Team
      - title: Full reset (Weekly + 5 hr)
      - description: Thanks for using Codex! You've been granted one free rate limit reset.
```

- Identifier
- `status`
- `reset_type`
- `granted_at`
- `expires_at`
- `available_count` for the account-level total
- `total_earned_count` if present
- `profile_user_id`, `title`, and `description` when present
- `redeem_started_at` and `redeemed_at` when present
- Any other fields only if they are explicitly present in the upstream response

For unsupported data, return:

- What was checked
- What official docs do expose
- Whether the internal `codex-rs` endpoint exists
- What is still unavailable
- The safest next step, usually account dashboard, support, or source inspection

## Guardrails

- Do not claim the internal endpoint is public.
- Do not infer coupon expiry or grant dates from the usage counter alone.
- Do not turn a general usage-limit question into a fake per-coupon inventory.
- Keep the answer short and explicit when the docs do not support the request.

## Reference

Read [official-docs-notes.md](references/official-docs-notes.md) and [internal-reset-credits.md](references/internal-reset-credits.md) before answering questions that mention `usage`, `limits`, `coupon`, `credit`, `reset credit`, or `promotional credits`.

## Script

Use `scripts/fetch_reset_credits.py` for the live account fetch. It reads the current Codex auth from the local profile and calls:

- `GET https://chatgpt.com/backend-api/wham/rate-limit-reset-credits`
