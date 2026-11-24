# Product post-deployment audit, 2026-11-24

The audit began on `main` at `652e0c31`. It compared the product contracts,
the source on that checkout, pinned-key GitHub release readbacks, and results
from Termux. Versions and runtime selections are tracked in
[CURRENT.md](../../CURRENT.md). Previously deferred cases remain in the
[roadmap](../ROADMAP.md).

## Component findings

| Component | Observed checks | Finding and disposition |
| --- | --- | --- |
| Wrapper, installer, release builder | Offline root workspace tests, installer and signing fixtures, official `codex --version`, verified Wrapper status, doctor, and live binary hash matched to the public signed archive | Wrapper and upstream selections were preserved. The installer fixtures include interrupted and rejected transaction paths. Physical device conditions remain in the roadmap. |
| `cdx` | Six unit and six CLI integration tests, live profile listing, release binary hash, and a handoff interleaving fixture | Consuming a ticket by reading and unlinking its public path could unlink a later request. A rename now claims the old ticket before reading it. Existing launchers keep their already running image. |
| Shared daemon and MCP libraries | Standalone tests for socket discovery and the MCP receipt protocol, plus calls from the compiled bridge binaries | Reused the existing daemon client and call receipts. |
| TermuxShell | Compiled stdio, receipt replay, timeout, truncation and live connector call. An Android process fixture held both write ends after the shell exited | The old call took 4.353 seconds and left two reader threads alive. The candidate returned in 0.370 seconds with one total MCP thread, kept captured output, and marked it truncated. The fixture reaped its own detached process. |
| TermuxCodex | Stateless bridge tests, authenticated status through the actual connector, and signed binary connected to a separate real Codex daemon | The installed image was a local build with a different SHA-256 from the signed tag. The source and its daemon/MCP dependencies match the existing signed source. |
| AppHost and Provider | AppHost Python, page-driver JS and Provider Rust tests, authenticated completion readiness, current operating image matched to the signed release | No source change. The connected account and running generation were preserved. The ignored official-Codex streaming fixture and authenticated coverage boundaries are recorded in CURRENT. |
| TermuxChatGPT | Standalone tests, installed connector status, and signed bridge connected to the isolated authenticated Provider | The installed image differed from the public signed tag. The current bridge source and shared MCP dependency match the signed source. |
| Tunnel runner and supervision | Recipe and supervision tests, signed bundle readback, and real runner PID fixture with no MCP child | The old helper reported `active` for a live runner with no MCP child. The revised helper reported `pending_activation` and also checks the child during restart admission. Runner bytes differed from the signed release. Both builds report the same upstream commit and patch version. |
| TermuxOrch | Seven unit and fourteen integration tests, actual connector Work list, and live relay and MCP binary identity | Both processes used the signed generation. The relay and ledger were preserved through the shared tunnel runner change. |
| Frontend and retired code | Git-tracked component inventory and normative architecture | The reserved frontend has no tracked implementation. Retired lineages and long-term checks remain documented. |

Thirteen baseline test batches passed. The Provider suite had one separately
qualified official-Codex fixture marked ignored in the default run. The
public readback statically verified eight signed releases against the pinned
Ed25519 key. Initially four installed binaries matched their public packages.

## Improvements

- `6e38d6a2` claims cdx handoff tickets before reading them.
- `acfd72ae` closes Shell MCP output capture after the command.
- `2ebbdcbc` verifies the live MCP child for runit status and activation.

## Publication and activation

| Component | Signed commit | Sequence | Archive SHA-256 |
| --- | --- | ---: | --- |
| [codex-ext](https://github.com/humtr/codex/actions/runs/45351625600) | `2ebbdcbc` | 7 | `8fc744adc35d1b995a1e520bd47d56ba9217c85481cd3b5e6c96e3f47392af52` |
| [Shell MCP](https://github.com/humtr/codex/actions/runs/45351625655) | `2ebbdcbc` | 5 | `1f556b683a1e7ac13d582c67298033d3b68a81362f7a0f87799323e6c6ee7020` |
| [Codex MCP](https://github.com/humtr/codex/releases/tag/termux-codex-mcp) | `f3c9334e` | 4 | `e911702c7519bd68646453798318a0c46d65fcf0d95b259a427f7ba184c0727a` |
| [ChatGPT MCP](https://github.com/humtr/codex/releases/tag/termux-chatgpt-mcp) | `f3c9334e` | 4 | `f5344eb46de658eb719f3f2d90c2d5a53f2392eb83cbbc938c3d83fe4d2c2eef` |
| [Tunnel runner](https://github.com/humtr/codex/releases/tag/tunnel-client) | `f3c9334e` | 4 | `f9a4cb74a7eea63ebf17ad5c51dd7230b786abfc90567b9351211dd7d9c0e887` |

The four tunnel services stopped before the common runner and changed bridge
selections moved. Each restarted service reported ready and had a live MCP
child whose executable matched the selected signed image. Provider and
Orch relay retained their process identities. The old tunnel and bridge
selections are retained under `previous`; the previous signed `codex-ext`
binary is under `ext/generations/0.1.0-76a5d3fd`. No package version changed.
[Tunnel and supervision CI](https://github.com/humtr/codex/actions/runs/45351612148)

After activation all eight installed executable hashes matched their signed
archives, and `codex doctor --json` reported healthy on `0.171.0`. The Shell
connector returned exit code 0 and replayed the same receipt. The Codex MCP
reported the existing `janmori101` daemon running through local stdio and the
connector. The first connector read produced no parseable result, and a
read-only retry succeeded. The ChatGPT connector reported authenticated and
ready, and Orch listed all four existing completed Works.

## Independent review follow-up

The three independent review commits were applied as `1789e029`,
`a1c780da` and `3c111174`. Follow-up commits `1a871fa1`, `e026e974`,
`ff2cafbf` and `c4f307b0` addressed issues observed in review and during
the actual device activation.

| Area | Final decision and evidence |
| --- | --- |
| `cdx` handoff | The launcher and writer use the same lock to serialize registration and admission. The launcher polls its generation-bound mailbox and closes admission before TUI teardown. Existing older launchers can still receive the legacy signal. The installed binary rejected a ticket with mismatched start ticks, resumed the requested thread exactly once with a fixture Codex, and removed registration and the ticket. |
| Shell MCP | The capture threads use one 100 ms deadline. When control-byte escaping exceeds the receipt limit, only the text previews shrink. The installed release settled and replayed both 65,536-byte NUL streams with exact Base64, returned both untruncated tails on clean EOF in 68 ms, and kept captured output when a 1,000 ms timeout killed the process group. Five separate MCP processes using one call ID produced one side effect. |
| Supervision | Coordinated activation stops the old service cohort before changing selection and starts the relay before the tunnel. A live logger caused the initial device cutover to stop before changing selection. The status parser now reads the service segment separately from the logger and has a regression test. The next attempt safely rolled back when the readiness probe used the old port. The successful attempt read the dynamic `health.url` on every check. An isolated Android `runsv` fixture exercised two actual services, a busy deferral, a successful cutover, and readiness-failure rollback while retaining the distinct previous link. |

Both release tags point to `ff2cafbf45dbc09c276011b98bbe69252f65bfb6`.
The downloads passed the pinned Ed25519 signature, outer archive checksum
and inner binary checksum before installation. The source-only
`runit_service.py` parser fix passed its separate CI workflow.
[Supervision CI](https://github.com/humtr/codex/actions/runs/45363745217)

| Component | Sequence | Archive SHA-256 | Installed binary SHA-256 | Retained rollback |
| --- | ---: | --- | --- | --- |
| [`cdx`](https://github.com/humtr/codex/actions/runs/45361685011) | 8 | `f05796403ee14e6c4ea1328493605a55c5f147d8abee01e257274902799d35f4` | `18df3e038d5f2520ddbb0db5d3c110637acd2ee28874c64ec3e76964e9d527be` | `ext/generations/0.1.0-2ebbdcbc/codex-ext` |
| [Shell MCP](https://github.com/humtr/codex/actions/runs/45361685187) | 6 | `43053a259413efbd14ffd40ca9734155d13ea1220da9afbe90f7f75091ad8984` | `9a0612a7e2005e5a894669ce9c26d5786024112c1a9786f6b76a1c2afc66cc67` | `mcp/termux-shell/previous` points to `0.1.0+2ebbdcbc` |

The follow-up audit downloaded and verified all eight public releases again
and compared them to installed bytes and the active service images,
including the four live MCP children. All six services were active, the four
tunnels returned `/readyz`, and the Provider and Orch relay passed their
local health endpoints. The other five services retained their PID and
start ticks through the Shell cutover and rollback attempts. All eight
rollback executables were present and executable. The four signed MCP
binaries completed local stdio initialization and listed respectively 1, 5,
6 and 6 tools, matching their advertised catalogs.

The repeat product suite ran 269 passing tests and one ignored Provider
fixture across 13 batches. It covered Wrapper and release building,
installer and signing, the daemon client and MCP kit, `cdx`, the four MCP
components, Provider Rust and JS, the AppHost manager and supervision.
An account-free physical Termux AppHost probe for `26.1117.41037` reached
one renderer in 5,681 ms and returned after the supervisor confirmed
shutdown. The production Provider reported `host_running=true` with its
existing supervisor. No authenticated account migration, live Orch
shared-ledger cutover, remote connector mutation, or physical UI
qualification is inferred from these checks. The earlier unparseable
connector result is unresolved at the remote boundary. Successful local
JSON-RPC and healthy transport logs only establish their observed paths.
