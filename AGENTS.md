# Publication branch

main contains the signed stable index and a caller pinned to accepted rewrite
source. Product code, contracts, current work and reusable release logic belong
on rewrite/rust-core. Read that branch's AGENTS and relevant source before
implementation; do not create another implementation or goal ledger here.

Change the workflow/source pins together only after applicable source acceptance.
Keep existing histories independent. Use ordinary commits; never force-push or
delete a published branch without explicit authorization. Do not overwrite Release
assets or change the stable index outside its signed, verified non-forced promotion.
A producer change is not permission for device activation or live data mutation.
