# Termux local execution preflight

**Applies to every agent, test fixture and local workflow on this Android
Termux device.** This is a practical checklist, not a new product contract:
[Core socket and generation authority](spec/delivery/TERMUX_COMPAT.md) and
[Native effect/temp ownership](spec/bridge/INPUT.md) are normative.
For latest device observations see [CURRENT](../CURRENT.md).

## Why previous live-TUI setup failed

The earlier isolated TUI fixture put the private Core/shared-server socket
under a **deep temporary tree**. Even with valid binaries, its pathname
exceeded the Unix-domain socket address limit; a short socket root allowed
startup. Separately, the subsequent `native_thread_owner_unavailable`
was caused by **different Native writer bindings** between the operating
Provider and isolated Core. Fixing one condition does not fix the other.

These are *preflight omissions and conflated test environments*. Core already
rejects overlong native socket paths; the old live qualifier did not catch
the invalid root before launching. Tests, scripts and docs had relied on a
generic `TMPDIR`/fixture path without separating socket root and evidence
root. Do not respond by extending an arbitrary timeout or changing the
installed Provider to match an unrelated fixture.

## Mandatory Unix socket path budget

On this platform `sockaddr_un.sun_path` stores **108 bytes including NUL**:
the complete absolute filesystem pathname must be **at most 107 encoded
bytes**, not 107 Unicode characters. Check with `os.fsencode(path)`;
`PATH_MAX` is irrelevant. Both of these paths must separately fit:

1. Core's real shared-server rendezvous:
   `<core-state>/servers/<configuration-namespace>/s` (qualified Core
   computes and validates its actual path).
2. The physical upstream Native socket:
   `realpath(TMPDIR)/<UID>/<64-character hex name>` for the FD-35/shared
   server. It must pass the encoded-byte test using the **canonical**
   root and numeric UID, not the original symlink spelling.

Use the installed short private Termux root `$PREFIX/tmp` (typically
`/data/data/com.termux/files/usr/tmp`) when correctly owned/configured.
**Never point the app-server `TMPDIR` at a deeply nested fixture,
workspace, CODEX_HOME, audit directory or experiment report root**. A long
workspace or report folder does not itself violate the socket limit unless
used to construct the socket path. Verify both physical and logical paths
before real model input; a passing Python preflight supplements, but does
not replace, Core's own ownership/namespace checks.

## Separate disposable outputs from runtime socket roots

- **Socket root:** short, private, canonical, owner-verified directory. The
  actual Core execution environment passes the required `TMPDIR`/FD-35;
  do not fake its parent with a symlink or overwrite an operating service.
- **Scratch/evidence root:** may live beneath an owned
  `$TMPDIR/codex-native/<work-id>/` or a long fixture directory, but **must
  not become the runtime socket root**. Keep separate per-work build targets
  and source-version proof. Create with exclusive owner-only permissions,
  verify no symlinks/aliases, and retain uncertain results.
- **Cleanup:** remove only definitively owned, finished disposable outputs.
  Never delete another task's subtree or all of `$TMPDIR`; never access
  Android sdcard, credentials or signed active/rollback trees.
- A temporary source checkout and Native work may use a long working
  directory. A fake short socket alias is not a substitute for verifying
  real ownership and configured execution identity.

## Core/Provider/TUI identity is a separate gate

- For the *installed product* live test, launch **the actual Core dispatcher**
  `$PREFIX/bin/codex`, not an embedded frontend helper. Inspect current
  activation, installed executable, selected profile and original owner.
- Read configured Provider executable/endpoint, Core path, HOME, CODEX_HOME,
  workspace, execution-home and socket/writer identity without logging secrets.
  Actual inference must target the **same Native writer** and its live Core
  binding. A Provider bound to the operating wrlab home cannot be used as
  proof for an unrelated isolated HOME/workspace.
- For fully isolated tests, isolate **both** Core/Native and the Provider
  binding, or remain zero-model/synthetic. Never assume a successful picker
  implies successful Native work admission.
- A disconnected SSE observer, transport error, HTTP 413 or timed-out ACK
  never authorizes a second submission. Query and reconcile the original
  operation/Native receipt; no replacement conversation or silent retry.

## Before committing a change that starts a local process

1. Inspect `git status`, branch/HEAD, selected home/workspace and concurrent
   work; reread AGENTS and the relevant SPEC module.
2. Check owner-only temporary roots, byte-length of both socket paths and
   actual Core/Provider writer identity; fail **before** first model input
   if any binding is unknown.
3. Run the expected-success fixture and an intentionally overlong-root
   **negative** test. Capture real process exit and preserved original result,
   not only a ready screen or test helper's success flag.
4. Record exact run SHA and evidence boundary in CURRENT; historical failures
   go to docs/legacy. No operating profile/service/release mutation without
   separate explicit authorization.
