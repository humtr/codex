# Provenance and dependency notices

Repository-owned `desktop_*` modules implement app/local-function/MCP transport,
correlation, lifetime and observation. Source integration preserves the experiment
history at `ab41733` rather than importing its old product snapshots.

The parser, Responses SSE/queue, tool mapping, turn identity and summary helpers
are reused from [miuuyy/codex-chatgpt-web](https://github.com/miuuyy/codex-chatgpt-web/tree/92a356fac2292e3af5a97ab7ba634edd8d38621e)
(exact source92a356fac2292e3af5a97ab7ba634edd8d38621e), with
`desktop-bridge-615.patch`. This code is not claimed as wholly original.
The installer retains effective dependency source, LICENSE and LICENSES when
bundling these helpers. The old Codex Native2 connector display name is not a
required runtime protocol identifier.

The optional development host also uses pinned proot support and noVNC assets;
their source qualification and payload/license retention remain in the installer.
License texts are retained under [licenses](licenses/bridge-LICENSE), including
[noVNC](licenses/novnc-LICENSE.txt) and [proot](licenses/proot-LICENSE).
Desktop application binaries, logins, profiles, tokens and rootfs/user data are
not vendored here. Source integration does not grant redistribution rights for
third-party desktop application binaries.
