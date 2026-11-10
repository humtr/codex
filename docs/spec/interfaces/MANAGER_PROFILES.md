# Manager contracts

> Normative module of [docs/spec/USER_INTERFACES.md](../USER_INTERFACES.md). Check [CURRENT](../../../CURRENT.md) for implementation/evidence.

## Manager contracts

This section defines the optional Manager. It does not weaken or
extend the Core completion threshold, and it does not make Manager a
prerequisite for ordinary upstream launch, Core doctor, update, rollback, or
fresh installation. Manager is an optional, separately qualified artifact
behind the existing `codex termux` boundary.

### Process and ownership boundary

Core selects Manager only from the signed, qualified generation and invokes
the artifact with a versioned handoff environment:

```text
CODEX_TERMUX_CORE_API=codex-manager-core-v1
CODEX_TERMUX_CORE_ENTRYPOINT=<validated stable Core entrypoint>
```

The retired `CODEX_TERMUX_CORE_REQUEST` and
`CODEX_TERMUX_CORE_OPERATION` environment values have no dispatch meaning.
Core does not parse or consume a repair request, even if those retired values
are inherited. Ordinary doctor/update/rollback/upstream argv retain their
normal behavior. There is no replacement repair request or hidden recovery API.

Manager validates the versioned API and Core entrypoint before doing work.
Its return path to Core is an `exec` of that entrypoint with ordinary upstream
argv whose first token is not the exact selector `termux`, `doctor`, or
`update`. Recovery uses the public Core commands directly. Manager cannot
address a generation path, activation state,
trust key, resolver, or journal directly. Core remains the final validator of
every requested route. There is no callback socket, network protocol, or
second state authority.

Manager receives no credential or session-content payload from Core. It may
inherit ordinary process environment needed for a child launch, but it must
not print or persist that environment. A missing, malformed, or incompatible
handoff fails before any Manager state mutation.

### Profile selection and isolated launch

Profile commands use this public grammar:

```text
codex termux profile list
codex termux profile current
codex termux profile create <PROFILE_ID>
codex termux profile delete <PROFILE_ID>
codex termux profile rename <PROFILE_ID> <NEW_PROFILE_ID>
codex termux profile default [PROFILE_ID]
codex termux profile use <PROFILE_ID> [--] [UPSTREAM_ARGS...]
```

`default` is the existing upstream default identity home. A custom profile uses
the derived path `~/.local/share/codex/manager/profiles/<PROFILE_ID>/home`.
Profiles are execution identities, not conversation owners: authentication,
user configuration, MCP/plugin credentials, model/provider caches, logs, and
shell/runtime snapshots remain profile-local, while local conversation state is
user-global beneath the canonical shared-state root `$HOME/.codex`.

`profile create` creates only the private profile directory, an empty private
account home and Manager metadata. It never creates canonical conversation
directories or copies/parses credentials or transcript bytes. Registration is
valid before the first launch; list/use validate the private home and exact
metadata rather than requiring prepared shared links. Core alone prepares and
validates execution topology before upstream runs, including direct supported
CODEX_HOME launches without Manager. Existing complete homes remain supported.
The compatibility topology links these profile-home names to their exact
canonical siblings beneath `$HOME/.codex`: `sessions`, `archived_sessions`,
`session_index.jsonl`, `thread-writer-locks`, `rollout-migrations`, `memories`,
`memories_v2`, `history.jsonl`, `installation_id`, and
`tui-thread-reference-capabilities`. Directory targets are real private
directories beneath the shared root; file links may initially be dangling only
to the exact canonical sibling so upstream can create the target atomically.
Core accepts missing entries or empty real directories for preparation and
rejects substituted, relative, chained or otherwise unexpected shared-state
links and nonempty conflicting entries rather than repairing/importing them.
Manager's accepted dotted profile IDs are also recognized by Core; declared
external homes retain their already-supported underscore/hyphen identifiers.
Existing legacy profile directories are not imported implicitly. Legacy
consolidation is a separate, one-time bounded compatibility migration with a
verified restorable backup and conflict checks. For each legacy user profile,
the migration selects at most the five most-recent distinct conversations by
last activity. Selection must be deterministic; if the legacy state does not
provide enough trustworthy information to establish that order, migration for
that profile fails closed rather than guessing. For each selected conversation,
the migration may inspect legacy state only as needed to normalize and carry the
dependency closure required for faithful current-upstream resume/history
behavior. Proven-identical copies of one conversation/thread identity may be
deduplicated; divergent state for the same identity is a conflict and must not
be overwritten automatically. Whole-SQLite-file replacement, blind `INSERT OR
REPLACE`, and persistent generic legacy-merger behavior are prohibited.
Non-selected legacy conversations remain unchanged in the verified backup or
source profile and are outside canonical import scope.

The bounded migration implementation must remain outside steady-state Manager/Core
session semantics. Its planning phase may read legacy SQLite read-only only to select
the bounded recoverable set, preserve explicit names, and reject unsupported
thread-scoped dependencies. It must not merge or write SQLite. The migration payload is
the selected authoritative rollout JSONL, verified by thread identity and content hash.
Upstream `thread/list` owns metadata read-repair, `thread/resume` owns paginated-history
materialization, and `thread/name/set` owns explicit-name restoration. Rebuildable
`thread_history_1.sqlite` projections and the dedicated diagnostic `logs_2.sqlite` store
are not migration payloads. A selected canonical rollout that is only a legacy-profile
symlink must be normalized to an identical regular canonical rollout before legacy
profiles can be retired.

Before upstream activation, an apply journal may roll back only rollout files or alias
normalization created by that apply. Once upstream activation begins and upstream may
have mutated canonical SQLite, partial payload rollback is prohibited; rollback becomes
restoration of the separately verified full canonical backup. SCS-6 therefore requires
writer quiescence plus a restorable full backup before live apply/finalize.

The steady-state architecture is deliberately thinner than that migration
exception. Manager profiles are execution identities; conversation persistence
and thread/session schema semantics belong to the official upstream runtime.
Manager's normal responsibility is profile registration/selection, bounded
profile-local environment/configuration and `exec` through the validated stable
Core entrypoint. Core owns physical shared execution-path preparation; upstream
owns the conversation store. Once
migration is complete, ordinary Manager operation must not routinely parse
SQLite or transcript contents, infer conversation ownership, or maintain a
parallel thread/session store or index. Introducing such behavior is an
architecture-regression signal unless an explicit, narrowly bounded
upstream-compatibility exception is added to this specification with executable
proof that the upstream-supported surfaces are insufficient.

`profile list` emits `default` followed by valid custom profile IDs, one per
line, in deterministic bytewise order. It ignores malformed profile records and
validates private profile registration rather than treating arbitrary
symlinks as profiles. `profile create <PROFILE_ID>` emits exactly
`created: <PROFILE_ID>` followed by one LF after the create-new transaction
commits; it emits no path, environment, credential, or upstream-state detail.
`profile current` emits exactly two LF-terminated lines, `current: <TARGET>`
followed by `source: <SOURCE>`. `<TARGET>` is `default`, a valid custom profile
ID, or `external`; `<SOURCE>` is `inherited` when the caller supplied a nonempty
UTF-8 `CODEX_HOME`, `saved` for a valid explicit preference, or `default` otherwise.
Empty and non-UTF-8 values provide no inherited selection; a saved preference
still applies, with native upstream default-home interpretation when it is absent.
Current reports the account home a
fresh ordinary launch would use, not an earlier child selection. Known external
homes are reported as `external`, without treating them as Manager registrations. It never reports auth identity,
token state, session bodies, paths, or arbitrary environment values.

`profile use` requires an existing profile, validates its private registration,
then `exec`s Core without selection-history writes and emits no
Manager-owned success output before Core runs. For `default` it explicitly sets
`CODEX_HOME` to `$HOME/.codex`, bypassing any persistent custom default; for a custom profile it sets `CODEX_HOME` to the validated
profile home. Manager removes an inherited `CODEX_SQLITE_HOME` override instead
of producing a parallel shared-store selection signal. Core derives the shared
root from the supported execution home and supplies a system `requirements.toml`
requirement that fixes `sqlite_home` to the same
canonical shared root and requires `local_thread_store_compression=false` and
`background_paginated_rollout_migration=false` where the selected
upstream execution topology still relies on incompatible profile-local
rollout maintenance locks. The requirements
file is Core-owned, is removed or replaced only through the Core marker
contract, and prevents a profile `config.toml` from silently splitting SQLite
or enabling those incompatible background writers. The original upstream argv
after the profile selector is preserved exactly. If registration validation fails,
Core is not launched. If execution topology is unsafe or conflicting, Core rejects
before upstream execution. A selection changes neither later ordinary launches nor
the caller's environment. Core uses the same native-equivalent home interpretation
for shared preparation and server startup, preserving the supplied environment.

The profile lifecycle contract below applies. Interactive selection is the
existing launch/TUI convenience, not an auth-migration or conversation-copy feature.
Cross-profile session copying is deliberately absent
because conversations are shared objects rather than profile-owned objects. A
missing/invalid registration is a non-mutating Manager validation failure. Core
prepares only the declared missing/empty compatibility entries during launch;
conflicting nonempty state is never migrated or repaired as a launch side effect.

### Explicit profile lifecycle and launch default

The profile commands extend execution-account convenience; they never own or
remove installation-wide conversation history. `profile default` prints exactly
`default: <TARGET>\n`. With a target it validates an existing registration (or
native `default`), atomically saves the explicit preference and prints the same
line. This is not selection history: `profile use` never writes it. Preference
format is exactly `codex-manager-default-profile-v1\nprofile\t<TARGET>\n`,
private mode0600 under Manager's private state root. Missing preference means
native default. Invalid/missing-target preferences never follow an untrusted path:
explicit default/current queries and lifecycle mutations fail; Manager's ordinary
launch preparation returns blocked instead of silently choosing native default.

A nonempty UTF-8 inherited CODEX_HOME always wins. Otherwise Manager resolves its
saved preference through the bounded `__prepare-launch-v1`/`__execute-launch-v1`
contract. Core never reads the preference filename or profile metadata. Manager
validates registration, sets CODEX_HOME for this child and execs stable Core with
original argv/TTY/streams/signals/exit behavior. `__profile-launch` remains an
internal Manager endpoint, not Core's preference-discovery mechanism. Unavailable
optional preparation leaves normal Core launch usable in its unchanged confirmed
home; there is no post-handoff retry. Core-only/update/doctor/termux and unsupported
daemon routes keep their defined behavior. No selector may be smuggled through
an internal endpoint. Bare startup discovery
still runs at most once, after selection. `profile current` adds source `saved`
when the explicit preference selects a fresh launch; inherited source remains
inherited. The retired state-v1 record stays ignored and unchanged.

`profile delete ID` deletes that validated custom profile's local login/config/
cache files and registration; `default` is never deletable. Exact output is
`deleted: ID\n`. Shared canonical conversation files and compatibility-link
targets are preserved, and deletion never follows any symlink inside the account.
The command itself is the explicit request; no automatic deletion or cleanup of
other accounts is introduced. `profile rename OLD NEW` changes only the registered
ID/path, preserving local file bytes, inodes and modes without copying auth or
session payload; exact output is `renamed: OLD -> NEW\n`. Destination collisions,
unsafe/malformed registration and reserved/default names fail without replacement.
A profile selected by the explicit default preference cannot be deleted or renamed:
first choose another default. No two-record default/rename transaction is needed.

Profile creation, preference writes and lifecycle mutations serialize with an
exclusive kernel lock on the existing Manager profiles directory. Manager owns
that registry coordination; Core execution does not lock or parse the registry.
For every existing owned upstream execution home, Core opens the actual directory
and holds a shared kernel lock on its inode across native runtime and shared-server
execution using FD36. The lease primitive knows no account ID, Manager path or
registration format. At the lease boundary, aliases/relative paths bind the same
actual inode; upstream still receives the original CODEX_HOME. This does not
broaden the existing shared-layout or server-path admission rules. A missing home
is left to the existing execution preparation/upstream rules, and a foreign-owned
home does not become a Manager-managed account through this optional lease.

The existing declared Manager-home path contract still requires a real private
owned home and safe ancestors before the generic lease is acquired; missing,
replaced or unsafe declared homes fail, never become fresh isolated identities.
The compatibility topology/SQLite binding for the three already-supported shared
home layouts remains Core-owned so direct launches still see canonical history
without Manager. This is a retained data-integrity contract, not account selection
or a reason for Core to parse Manager metadata. No new binding registry, state
marker, user-data migration or second conversation store is introduced.

The home lease closes the mutation race: lifecycle requires its exclusive lock;
launch takes the shared lock and revalidates the opened inode against the selected
path before use. A registry-only mutation for another account cannot block an
otherwise safe existing-home launch. Core provides this execution lease only;
it does not implement deletion, rename or default policy. Manager
requires the exclusive home lease before any rename/delete and additionally
rejects existing old-generation live Core server bindings for that home. No user
server is stopped automatically. For pre-lease runtime processes, bounded same-UID
process metadata identifies CODEX_HOME/cwd/open account handles; only the execution
home field may be retained transiently, never unrelated environment values,
credentials, argv or session content. Unreadable/ambiguous relevant processes
reject mutation rather than claiming idle. Default selection may change while
work runs; existing work retains its account and authentication.

An execution-home lease proves directory use, not conversation writer ownership;
it never authorizes takeover, writer-lock removal or stopping another turn.

Core's generation-retention lease must occupy a descriptor above the reserved
33–36 range. Installing an execution-home lease under descriptor pressure must
not release generation protection before the final kernel exec.

New registration metadata is exactly `codex-manager-profile-v2\n`, without an ID
already owned by the directory name. Existing exact private v1 metadata remains
valid; rename first atomically replaces v1 by v2 while OLD is still valid, then
uses create-new directory rename and synchronizes the parent. Either observable
registration remains usable after interruption. No duplicated ID or journal is
introduced. Delete first atomically moves the validated account into one private
hidden `.delete-*` sibling under the held locks, synchronizes the directory, then
removes only that tree without following links. Before that move, a non-following
preflight rejects more than 65,536 entries or a directory depth above 128; an
unreadable tree fails before unregistering the account. Interrupted cleanup may leave
hidden private residue; it is not a profile, never used for launch, and a failure
is reported without pretending deletion of shared state or other profiles.
Version rollback never reverses explicit profile lifecycle actions. Managers
predating this extension ignore the preference and do not list v2 registrations;
account bytes and Core direct CODEX_HOME execution remain independent of metadata.
