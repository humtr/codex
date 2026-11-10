# Release execution

> Normative module of [docs/spec/DELIVERY.md](../DELIVERY.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

## Release execution

One maintained release workflow owns construction and publication. main is a
small immutable-source caller, with no second implementation or manually mirrored
producer. The caller selects an accepted exact implementation commit; nested
publication logic comes from that same source. Job-specific permissions separate
read-only construction/smoke from signing/public mutation. Signing secrets appear
only at the signing boundary. A manual publication needs explicit authorization;
scheduling authorizes only a qualified strictly newer upstream pair. An explicitly
authorized current-version rebuild uses its exact qualified upstream/archive and
normal next sequence, with all ordinary integrity/publication gates intact.

Remove completed one-shot acceptance/deployment code paths, old mutation workflows
and experimental preview construction. Existing immutable migration releases and
required consumer compatibility remain ordinary product support, not executable
historical producer modes. Preserve normal signature/inventory verification,
LKG continuity, candidate site size, complete public readback, disposable consumer
proof and non-forced exact-parent CAS. No public release or installed activation
is implied by a development-plane change.
