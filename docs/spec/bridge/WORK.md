# Bridge lifecycle and interoperability

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

## Bridge lifecycle and interoperability

### ChatGPT-Codex bridge architecture and work boundaries

Sections4.5–4.6 define the target ownership and user-flow contracts. Provider
adapts source/transport and external admission; Native owns the conversation,
effective route/context, execution and committed outcome. Existing work-control
records are implementation evidence to review, not an independent task authority.
Core provides runtime trust/lifecycle; frontend and optional Manager consume
Native state and submit intent. No second executor, transcript database, scheduler
or planning model is introduced. CURRENT distinguishes this design from shipped
behavior and names incomplete interfaces. Historical audits do not freeze the
earlier boundary allocation.

Historical architecture reviews do not determine the current contract.
Earlier initiative labels are not product completion levels.
[CURRENT.md](../../../CURRENT.md) owns implemented, qualified, published and
activated status. The following
contracts include the delivered bind/unbind/join foundation and the broader
entrance-equivalence target; API delivery alone does not qualify every target
scenario. See CURRENT's acceptance matrix and work order for the remaining scope.

#### Work recording scope and entry selection

The workflow names are fixed as **standalone — 단독 개발** and
**integrated — 통합 개발**. Standalone means ChatGPT standalone development;
integrated means the common task/tool/control/history/orchestration workflow.
Entrypoint, model and inference initiation do not redefine these modes.
Provider remains the component name, not a workflow mode. **MCP** and
**Function** are tool-delivery routes, not workflow modes. Function is the
user-facing name of the existing `local_function` transport through AppHost;
`local_function` remains its internal protocol identifier. Neither transport
selects standalone or integrated, creates a Native work by itself, or changes
resource ownership. Do not retain direct/provider or current/managed as workflow
aliases.
CURRENT records implementation and qualification; this contract does not claim
that the installed MCP already exposes a threadless local-tool surface.

- ChatGPT local tools return actual file/command results to the initiating Chat
  without creating Native threads/turns or copying user/assistant messages into
  Codex conversation history. Reusing the installed Core executable does not
  require a Native conversation, a model catalog or Codex model authentication;
  that API fact alone does not qualify a backend as standalone development.
- Integrated workflow owns all participation in Codex tasks, Native tools and
  orchestration: common instructions, work/turn admission, permissions,
  execution/control, public history, CLI projection and completion. A ChatGPT
  response attaching to existing Codex work uses integrated workflow too.
- Codex CLI entry and ChatGPT entry into integrated mode use that same common
  work admission/execution/event/result lifecycle. Differences in incoming
  transport and final display destination do not create a second development
  workflow. Standalone never serves as a bypass into Native work/tools.
- A new unbound ChatGPT task defaults to local tools. A Codex-origin input or a
  verified selection of an existing Codex task retains its Native work. One
  initial selection persists across ordinary follow-ups; repository paths,
  titles, incidental mentions and a global last-work selection cannot establish
  that relationship. Several tasks in one Chat need distinct selected contexts.
- Users may select or change the scope through explicit instructions or a
  supported setting/UI. Do not require another confirmation of an already clear
  instruction. Natural-language initial selection is model-assisted behavior,
  not a guarantee that unexpressed intent will always be classified correctly.
  A standalone lock must be enforced by server configuration or a verified
  user-selected profile/UI: a model argument alone cannot lift that lock.
- A scope transition affects subsequent operations and hands over necessary
  instructions/results without replaying previous effects or retroactively
  importing/deleting history. Existing jobs, approvals and controllers remain
  owned until their actual transition/Stop boundary settles.

The Provider owns scope admission, retained selection, authorized access and
uncertain-delivery reconciliation. Core/upstream own actual file/process I/O;
Native owns conversation execution only for selected Codex work. MCP and renderer
local functions use those same operation semantics and return to the original
caller. Keep one implementation per responsibility; two advertised profiles do
not justify two providers, executors or conversation stores.

A single MCP with explicit task contexts is the selected implementation design.
Linux-app actual-model calls establish conversation correlation, including across
MCP/app restart, but the same anonymous session also spans distinct saved branches.
Therefore a conversation-wide last-selected mode/work cannot be the dispatch key.
Retain the existing work/controller for Native calls; local operations carry their
own authorized workspace context. Correlation can find eligible contexts and
present defaults, never silently choose between unrelated tasks or branches.
Public enforcement/read-write/quality acceptance remains required before delivery.
Fixed local-tools/Codex-work profiles remain optional explicit entry/standalone-lock
surfaces over the same implementation. Making both visible does not itself
guarantee correct automatic selection. Do not vary registered tool lists on each
turn to hide an unresolved scope decision.

Host-provided anonymous session metadata may correlate calls only within its
verified lifetime and authenticated connection scope. It is not the Chat URL's
conversation ID, original message/branch provenance, authorization or user scope
selection. An actual model tool call and a manually issued Linux SDK tools/call
have different metadata: the latter need not contain a conversation session.
Missing/conflicting metadata requires a verified explicit context or fixed
profile, never inference from another Chat's last selection. Keep authenticated
connection scope separate from host subject/organization/session hints. A branch
edit cannot automatically inherit another branch's latest selection from this
anonymous ID. CURRENT records observed clients/lifetimes; new clients, account
changes and unresolved fork semantics need their own qualification. Retain the
separate original-source verification contract.

Native work allocation and task execution are separate operations. The declarations
and pre-effect validation share action-specific argument requirements: allocation
(create/resume/fork), join, queue, chat_input and source_join require a stable
nonempty call_id. Allocation does not accept task/model/effort/collaboration-mode
arguments; send actual user instructions on start/join instead of silently ignoring
them. Assignment budgets/parent/dependencies require an explicit role; ordinary
work need not invent an assignment budget. Expose these conditions in the schema
and field descriptions. Missing identifiers and misplaced task fields return
cause-specific pre-effect errors with correction guidance. Reads do not gain an
unused call_id requirement. An argument rejection does not justify changing the
user's integrated intent to standalone development.

Already-running ChatGPT inference can attach to Native Core without a separate
Provider-owned ChatGPT AppHost. Host status/ensure concerns that optional app,
not Core readiness or the validity of work-allocation arguments. Describe this
boundary in the exposed tools. A model-reported fallback is a failed workflow
qualification even when its standalone file edit/test succeeds.

The integrated MCP work list may mark each row `chat_correlated` when its call
carries host `_meta["openai/session"]`. Derive the private association from the
persistent authenticated service owner and session/subject/organization tuple;
never store raw host identifiers in that association. Record it only after a
successful explicit create/resume/fork/join/ownership selection or accepted
attachment. Inspecting work alone does not select it. Missing metadata leaves
the marker absent; another Chat/account/service has no borrowed last selection.
Several rows may be correlated, including across branches. The flag is a discovery
hint only, never a controller, branch selection, source receipt or default dispatch.
Keep metadata outside model arguments and reject malformed consumed metadata before
effects. A failed optional association write must be reported alongside the
original successful Native outcome, without resubmitting or hiding that outcome.
MCP restart/credential rotation retain associations within the same journal owner;
local functions retain their verified explicit lane/work identity and do not
invent Chat metadata. This adds no conversation transcript or workflow alias.

Threadless I/O must preserve authorized read/write development, relevant workspace
instructions and applicable approval/reviewer behavior. Upstream fs RPCs do not
inherit a cwd path grant, and command/exec process IDs are connection-local process
controls, not durable idempotency keys. Never expose raw APIs as equivalent to
Native tool admission without qualifying these differences. Reuse the existing
operation reconciliation responsibility: uncertain mutation delivery must not
trigger automatic redispatch. Necessary operation/control records are not a
second Chat transcript store. Stop, streaming, PTY, disconnect and reconnection
must be qualified on the actual path. Local-tool completion produces no Native
work-completion event; do not implement this by merely suppressing notifications
while still creating unwanted conversations.

Standalone local admission uses `execution.standalone_permission_profile`, an
explicit Core permissions-profile ID chosen by the operator, never a model tool
argument. Absence disables local execution. Core validates that profile and its
managed requirements; cwd is a working directory, not an additional path grant.
Commands, file edits/patches and tests use `command/exec` under that same profile;
raw fs RPCs are not exposed. This is a standing configured grant, not Native
turn approval/reviewer parity: do not claim a Native approval event occurred.
The local context returns the workspace, permission profile and instructions to
read applicable workspace guidance before effects. No model or effort is chosen
by these tools and no independent inference is submitted.

`local_context`, `local_execute` and `local_process` share one Rust operation
backend. Context IDs bind the service owner and configured execution scope;
commands require that explicit context and stable call_id. Execution captures
complete stdout/stderr as private operation output, with byte-offset paging and
base64 for exact bytes. A returned process ID confirms admission, not OS spawn
or exit. Dedicated connections isolate each command's stdin/PTY/Stop and lifetime
without starting a separate Core server per command. Deferred Core command replies
are not cut off by the ordinary short RPC observation deadline. Output loss or
unknown transport delivery is explicit, never successful completion or an
automatic command replay. Exact retries return the original process/result even
across service restart; an unsettled old operation cannot restart itself.

Process controls have their own stable call_id and preserve original Core replies.
A terminate acknowledgement is not exit confirmation; only the original exec
result confirms completion. Validate the streamed command response shape before
marking exit; an invalid response remains unknown with its original reply retained. An early control can be explicitly rejected before
Core has registered the command; this is not a successful Stop. Connection loss
fences further controls on that handle and does not borrow a replacement
connection. Provider shutdown releases its own command connections only, not
shared Core servers or unrelated Native jobs.

`execution.workflow_lock=standalone|integrated` optionally fixes the server's
workflow surface for its whole lifetime. Standalone requires an explicit local
permission profile, excludes Native work/tools and does not recover/rotate their
controllers. Integrated excludes local tools. Neutral host/project/Chat inspection
remains available. The same admission guard rejects calls hidden by the profile;
model arguments and host correlation cannot lift it. Workflow locks govern Bridge
tool/inference admission; Core's selected permissions separately govern programs
and file access inside commands. Commands intentionally starting Native development
or Codex orchestration belong to integrated work. An unlocked service exposes
both configured surfaces and retains explicit contexts without global last-work
dispatch. Integrated model-owned local-function catalogs exclude standalone tools;
a standalone function consumer must use the same local backend and grant, never
pretend to be an integrated Native turn.

These optional settings preserve serialization of prior configurations when absent.
Rollback to a pre-standalone Provider requires the matched prior configuration
(or removal of the two new execution settings while stopped), as well as the
existing explicit inference-state downgrade. Preserve local operation/output
records: an older Provider must not convert them into Native conversation work.
