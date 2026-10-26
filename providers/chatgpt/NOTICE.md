# Current Provider source and third-party dependency notices

The active optional ChatGPT Provider is implemented by repository-owned Rust
modules and an app-local JavaScript bridge. It does not load or require the
retired JavaScript adapter, its parser, or its transport runtime. Earlier
experimental implementations remain in Git history rather than the current
Provider package. Historical source provenance is not an active dependency.

The Provider may use separately provisioned PRoot, Xvnc and noVNC components,
with the applicable notices and license terms. The source package retains
[noVNC](licenses/novnc-LICENSE.txt) and [PRoot](licenses/proot-LICENSE) notices.
ChatGPT desktop application binaries, logins, profiles, tokens and user/rootfs
data are not bundled. Source integration does not grant redistribution rights
for third-party desktop application binaries.

Compaction control uses Provider-owned public continuation data and an issued
text-summary identifier. The Provider does not bundle an upstream summary
preamble or infer native acknowledgment from copied template wording. Native
Codex owns its history and any preamble it adds to a returned summary. Its
separately installed distribution retains its own applicable notices.

Previously signed Provider releases retain their immutable source assets.
Their exact historical file inventories remain verifiable for rollback, without
adding retired adapter/template files or their notices to new releases.
