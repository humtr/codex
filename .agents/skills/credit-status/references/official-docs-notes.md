# Official Docs Notes

This audit checked the public OpenAI docs for usage-limit and coupon metadata.

The review did not find a public API surface for:

- Coupon-level grant history
- Coupon expiry timestamps
- Coupon issuance timestamps
- Other hidden per-account coupon metadata

Use the docs to confirm the existence of usage limits, but do not assume a public endpoint for coupon records.

Relevant pages:

- https://developers.openai.com/api/docs/guides/rate-limits
- https://developers.openai.com/api/docs/guides/admin-apis
