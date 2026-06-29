# Internal Credit Status

This reference describes the internal `codex-rs` surface the user reported.

Endpoint:

- `GET https://chatgpt.com/backend-api/wham/rate-limit-reset-credits`

Expected record fields:

- `id`
- `reset_type`
- `status`
- `granted_at`
- `expires_at`
- `redeem_started_at`
- `redeemed_at`
- `profile_image_url`
- `profile_user_id`
- `title`
- `description`

Top-level fields:

- `credits`
- `available_count`
- `total_earned_count`

Usage rules:

- Treat the endpoint as internal, not public.
- Prefer this endpoint over `/usage` when the goal is to inspect per-credit metadata.
- If the endpoint is missing in the current source tree, fall back to the public docs and say that the detailed inventory is unavailable.

Preferred display shape:

```text
조회 결과:
  - available_count: 3
  - total_earned_count: 0

  1. RateLimitResetCredit_...
      - status: available
      - granted_at: 2026-06-12T02:12:33.195949Z
      - expires_at: 2026-07-12T02:12:33.195949Z
      - redeem_started_at: null
      - redeemed_at: null
      - profile_image_url: https://openaiassets.blob.core.windows.net/$web/codex/codex-icon-200.png
      - profile_user_id: Codex Team
      - title: Full reset (Weekly + 5 hr)
      - description: Thanks for using Codex! You've been granted one free rate limit reset.
```
```
