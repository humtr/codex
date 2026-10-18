# Provenance and dependency notices

Repository-owned `desktop_*` modules implement app/local-function/MCP transport,
correlation, lifetime and observation. Source integration preserves the experiment
history at `ab41733` rather than importing its old product snapshots.

The retained JavaScript research/legacy parser, Responses SSE/queue, tool mapping,
turn identity and summary helpers are reused from [miuuyy/codex-chatgpt-web](https://github.com/miuuyy/codex-chatgpt-web/tree/92a356fac2292e3af5a97ab7ba634edd8d38621e)
(exact source92a356fac2292e3af5a97ab7ba634edd8d38621e), with
`desktop-bridge-615.patch`. This code is not claimed as wholly original.
The installer retains effective dependency source, LICENSE and LICENSES when
bundling these helpers. The old Codex Native2 connector display name is not a
required runtime protocol identifier.

The production-candidate Rust parser, Responses service, lane journal and retained
MCP HTTP/stdio return modules are repository-owned implementations. The existing
independent JavaScript transport informed behavioral regression cases; its source
and provenance remain preserved. They do not depend on the legacy parser runtime
or require its connector display name.

The optional development host also uses pinned proot support and noVNC assets;
their source qualification and payload/license retention remain in the installer.
License texts are retained under [licenses](licenses/bridge-LICENSE), including
[noVNC](licenses/novnc-LICENSE.txt) and [proot](licenses/proot-LICENSE).
Desktop application binaries, logins, profiles, tokens and rootfs/user data are
not vendored here. Source integration does not grant redistribution rights for
third-party desktop application binaries.

`native-summary-prefix.txt` retains the upstream Codex compaction prefix from
OpenAI's Apache-2.0 source at83086188ae0614bd882818ff1aa23f3194962012,
`codex-rs/prompts/templates/compact/summary_prefix.md`. It is an upstream wire/history
fixture (license retained as [upstream Codex](licenses/upstream-codex-LICENSE)),
not a model-family list or a translated ChatGPT application component.
Rust owns checkpoint admission and acknowledgment independently. Existing
`desktop_protocol.mjs` guards inform equivalent Rust regression cases; their
original provenance above remains preserved.
