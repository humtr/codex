# Manager TUI frontend

SPEC.md owns the generation, command and update contracts. The native patch adds
/switch to exact official0.163.0 source83086188ae0614bd882818ff1aa23f3194962012;
upstream retains every existing command and owns conversation UI/title composition.
Manager owns execution profiles and foreground identity. The0.163 candidate must pass native/device gates before publication. Official runtime remains
the sole execution backend.

`native.patch` contains integration changes to existing upstream files. The
standalone Manager TUI implementation is ordinary source under `src/`, including
profile/task views, model routing, permission adaptation and Android window
callbacks. Run `python3 frontend/manager-tui/apply.py UPSTREAM` once on a disposable
pristine checkout; it preflights collisions and patch applicability before writing.
Do not apply the patch alone. Qualification binds the complete owned source tree
and assembler as well as the patch, and retains the assembled diff as evidence.

`termux_backend_catalog::bind` owns catalog startup selection after upstream loads
configuration. `termux_chat_models::reentry_route` plans provider/profile arguments;
the App hook retains only work-state admission, persistence and re-entry. These
remain optional frontend code; Core has no dependency on these sources. The
provider re-entry behavior remains unchanged by this split.

The signed helper's hidden `--termux-backend-capabilities` query advertises
`codex-termux-resolved-startup-v1` before loading config or initializing a terminal.
Core uses an exact bounded response to defer backend creation. With
`CODEX_TERMUX_BACKEND_STARTUP=resolved-v1`, the adapter requests only the
upstream-resolved catalog, or `__app-server-default-v1` when no catalog is selected.
An old Core still eagerly supplies its socket; an old helper still receives the
same eager behavior from a new Core. The signed helper identity/inventory is
unchanged. No provider policy or profile TOML parsing moves into Core.

The maintained hosted workflow checks pristine command collisions, unchanged
external dependencies, exact executable version, actual native tests and lint.
The release builder owns FD adaptation and signed generation publication. This
unsigned source build alone does not admit or install a stable release. Historical
preview assets remain protected until their clients exit.

The frontend passes its upstream-resolved model catalog to Core's existing
backend coordinator before local startup. Exact catalog bytes, execution home
and signed backend identify reuse; native/default siblings keep their own socket.
Manager discovers both native and catalog-bound writers. The42-test candidate
gate includes catalog handoff, socket-output rejection and Core daemon ownership
alongside profile, permission, window and upstream command regressions. Separate
upstream-feature gates exercise0.162's managed worktrees, shared pinning, copy/
scroll/links, startup defaults, provider capabilities, CodeMode settlement/search,
Retry-After and patch line endings. Every topic requires an actual passing test;
source inclusion or a nonempty aggregate suite does not qualify a missing topic.

Local resume carries the supported Termux permission selection even when Core's
system default supplied it. Saved approval policy/reviewer remain independent.
Explicit restricted policies fail before local thread/turn requests; they are
never silently converted to full access. Side forks inherit actual parent settings.
`scripts/qualify_permission_lifecycle.py` uses a private fixture signer, an inert
local model and real Android native commands/reviewer requests. It seeds saved
workspace permissions, stops the owned server, resumes through the actual TUI and
opens /btw. Legacy and named configurations require three commands and three
guardian reviews each. No real model inference, credentials or production state
are used. Pass the raw qualified frontend with `--frontend`, an installed Core
with `--core` and a private output path with `--report`; `--named-permission` checks
named selection, and `--expect-stale-resume` qualifies a pre-fix negative control.
Pass `--manager` to qualify a candidate Core and Manager together rather than
reusing the installed Manager in the disposable signed fixture.
`--expect-single-backend` proves fresh launch/cold resume/side chat create only
the selected namespace; `--native-catalog` exercises the native/default path.
`--catalog-layer user|cli|project|profile` checks upstream configuration resolution
through the real frontend. Syscall observation attaches only to the exact owned
server after resume, before /btw, so tracing bundled-file extraction cannot distort
startup deadlines. The actual fork response is still required for acceptance.
