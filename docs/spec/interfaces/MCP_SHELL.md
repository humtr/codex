# Independent Termux MCP shell

> Normative annex of [SPEC](../../../SPEC.md); [INDEX](../INDEX.md) is navigation only and [CURRENT](../../../CURRENT.md) is observed status.

`termux_shell` is the single independent Termux app-UID host shell tool. The
standalone stdio worker handles it before the existing Provider proxy, preserving
all existing Codex tools and their common MCP annotation policy. There is no
shell-specific annotation exception and no additional receipt/status/recovery
tool. Core, Provider inference and Native shared-effect admission are not execution
dependencies. Recovery is a primary use, not a restriction to recovery commands.

The caller supplies a nonempty bounded `call_id` and command/cwd/timeout arguments.
The existing private receipt owner atomically creates an execution intent before
spawn. Same ID and exact arguments return the original receipt, including started
or uncertain state, without another process. Reusing an ID for different input is
a conflict. New fingerprints bind raw supplied arguments; legacy receipt
fingerprints remain readable without executing a historical call. Receipt parsing
validates schema/state/outcome and rejects aliases, unsafe ownership/modes and
oversized data. Creation and completion synchronize the file and parent directory.

Execution uses a private process group. Timeout/cancellation terminates the owned
group and preserves bounded already-captured stdout/stderr and actual exit/signal
information where available; cancellation before an outcome commit stays uncertain.
Ordinary surviving group children are terminated on command exit. Explicitly
detached sessions are not silently claimed as managed Native jobs. No caller may
infer effect rollback from process termination. After a disconnected response,
query the original ID first; do not blindly submit a new ID. Concurrent identical
calls cannot execute twice, and separate call results retain their own identity.

Commands are bounded to32KiB, each output stream to64KiB and persisted receipts
to512KiB. Truncation is explicit; receipts remain private and are not copied into
public diagnostics. The shell is a tool transport, not another AI engine, Native
thread/approval/controller database or conversation store. Native operations keep
their existing ownership, approval and result authority. Host access preserves
user-authorized scope, unrelated work and Termux platform/storage constraints.

Source/unit and isolated worker tests are distinct from actual connected ChatGPT
MCP round trips. Connector refresh and operating worker installation are separate
operational actions; a source update does not imply either occurred.
