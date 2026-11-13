# Development task: persistent runit generation activation

Status: OPEN. Owner: Codex for Termux Core/Provider/tunnel lifecycle integration.
This is an implementation task, not proof that automatic service rebinding exists.
The behavior contract is `docs/spec/bridge/OWNERSHIP.md`, with release and
qualification contracts in `docs/spec/delivery/QUALIFICATION.md`.

## Problem and user-visible target

An owner has explicitly opted the Provider, tunnel client and optional Host keeper
into existing Termux `termux-services`/runit services. Their `run` scripts source
a private environment file containing `CODEX_PROVIDER_BIN`,
`TUNNEL_CLIENT_BIN`, profile and state references. `codex provider update chatgpt`
selects a verified, immutable new payload, but does not update that environment
or restart the supervised image. The former Provider therefore keeps serving
obsolete HTTP endpoints even after the selected generation is updated. Tunnel
source-build output is similarly separate from the active service binding.
The user requires persistent, automatically reconciled service generation
selection on every authorized update or rollback, without re-enrollment or
manual path editing, while preserving sessions and service recovery on reboot.

## Design assignment

1. Inventory the actual ownership and consumers: the `install.sh` bootstrap,
   Core Provider installer/updater/rollback, optional tunnel builder/install and
   activation owner, Provider `service.startup=supervised`, existing shared runit
   environment plus Provider, tunnel and Host-keeper service `run` commands.
   Decide precisely which installed component owns each selected generation
   and which routine reconciles the already-opted-in service; do not invent a
   second updater, supervisor, registry or independent background controller.
2. Define a single typed, versioned local *service activation reconciliation*
   contract. Given verified selected generations, update only the owned paths
   in the existing private runit configuration. Prefer stable service commands
   resolving the signed current selection at startup over hard-coded versioned
   executable paths, provided ownership, signature/digest verification and
   reproducible rollback remain intact. Never trust an arbitrary symlink,
   stale executable, environment override or unchecked file.
3. Make update, explicit install, rollback and initial explicit service enrollment
   converge on this same path; account for `codex update --core|--full`, Provider
   update/rollback and the independent tunnel source-build/installation channel.
   Do not enroll a device in new services simply because Core or Provider is
   updated: runit remains opt-in. When opted in, service-binding reconciliation
   must persist across reboot, `runsvdir` recovery and future updates.
4. Coordinate independent Provider, tunnel and Host-keeper dependencies. Confirm
   the old Provider and tunnel identities, exact activity/leases, app-host owner,
   native effects, stream obligations and rollback assets. Permit an in-place
   verified switch only after a safe boundary. Preserve the old HTTP listener
   endpoint and tunnel profile/remote tunnel; never create duplicate services,
   simultaneous tunnel owners or new credentials/logins. If activity cannot
   be settled or readiness is unknown, report `pending_activation` with the
   selected and running generations, leaving the old service operational.
5. Persist configuration changes atomically under a private owned directory:
   validate exact old content and file ownership/modes; write, fsync, rename
   and fsync its directory; retain compatible rollback values. Restart only
   the affected owned runit service, wait for confirmed PID/start/executable
   change and authenticated readiness, and independently verify endpoint
   protocol capability and tunnel connection. A `sv restart` timeout is
   *unknown outcome*: inspect original service identity before any retry.
   On failure, recover the last compatible binding without rewriting signed
   generations or replaying model/native effects.
6. Surface three separate states through `codex provider status chatgpt`,
   `codex termux version`/doctor or appropriately scoped subcommands:
   `selected generation`, `actually running service generation` and
   `activation pending/error`. A successful package installation alone must
   never claim that a stale Provider service or tunnel was upgraded.
   Keep the normal standalone Provider path functional without runit.
7. Implement focused tests for first enrollment, un-enrolled install, no-op,
   S26-to-S27 and tunnel version upgrades, rollback, missing/invalid old
   binary, mismatched signature/digest, stale/non-owned PID, busy Native work,
   concurrent updates, interrupted atomic writes, stopped supervisor, reused
   port, occupied new port, runit restart timeout, reboot recovery, and wrong
   local provider/model routing. Preserve old-format configuration readers.
8. On the *actual* Termux device, verify the selected signed binary matches
   the running runit child image, no duplicate service or tunnel, AppHost
   account/session continuity, old stable/legacy Native endpoint contract and
   negotiated stable capability routing, and reversible authentic update.
   Use explicit tool receipts, no private-token/log exposure, no sdcard and no
   unnecessary re-runs of unchanged long CI. Final release needs standard
   exact-source qualification and separately authorized signing/publication.

## Integration disposition and current observation

This open future task has been brought from branch
`work/tunnel-termux-v0.0.20-20261113` into the continuing development line.
Its implementation is **after** the actual orchestration/0.167.2 acceptance
and documentation phases of [HANDOFF](../../HANDOFF.md). Existing manual
runit repair is preserved; this document does not authorize service restart.

At 2026-11-13 14:22 KST, read-only examination found the Provider runit
child PID 21766 executes
`provider-chatgpt-v0.167.2-0ba0589-s27`, whereas the filesystem Provider
`current` link points at the separately named
`provider-gpt-v0.1.0-df8a8c9-s27`. Both have an S27 suffix. The meaning of
the legacy pointer must be established from actual consumers before asserting
an update or current-selection defect. The operating image is not the legacy
path. The Provider, tunnel and Host-keeper services all reported `run`; no
reboot, automated rebind or rollback occurred in this check. The future
acceptance must independently prove selected-generation vs running-image
convergence and intentional legacy-link compatibility, not just PID existence.

## Current motivating evidence (2026-11-13 KST)

- Full #167, optional Provider S27 selected. Its runit environment initially
  still pointed at the November 9 standalone qualification executable and served
  404 for all Native paths; the selected payload itself was not defective.
- The authorized, one-time manual service-binding repair changed only
  `CODEX_PROVIDER_BIN` and supervised Provider from PID 12629 to S27 PID 21766.
  Authenticated `GET /v1/native/capabilities` then returned HTTP 200, and the
  stable protocol-4 negotiated Native URL reached request parsing; no model
  turn was submitted by that diagnostic.
- Tunnel was enrolled as `codex-chatgpt-tunnel`. The owned, one-time
  runit environment update and confirmed restart now execute the Android/arm64
  `v0.0.20` patched binary (PID 16316), preserving the existing profile.
  The optional Host keeper was also restarted to adopt the selected S27 path.
- This incident repair is *not* the requested automation. Close this task only
  after real update/reboot/rollback acceptance of the automatic reconciliation,
  not merely a one-off edit of a runit environment variable.

## 2026-11-13 15:17 KST operating readback

Runit Provider PID **8454** was verified through `/proc/8454/exe` to
run `provider-chatgpt-v0.167.2-0ba0589-s27/codex-chatgpt-provider`,
not the legacy-named S27 pointer. Tunnel and Host keeper runit
script processes were PID **8450** and **8457** respectively.
Read-only observation does not establish automatic service generation
reconciliation on install/update/rollback or reboot. The task remains
OPEN; no service or installed generation was modified.
