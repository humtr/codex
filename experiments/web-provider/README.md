# Termux ChatGPT Web experiment

This separate branch follows repository AGENTS.md. Production configuration,
profiles, conversations and services remain authoritative and unchanged.

Selected bridge: miuuyy/codex-chatgpt-web at
92a356fac2292e3af5a97ab7ba634edd8d38621e, version6.1.5. Frozen Bun dependencies
and official Android/aarch64 Bun1.4.0 run directly on this phone. The official
Bun archive SHA256 is42544d7438bb92c7e7df7d30b9a5858cb7a834636608e5b850f59138283567fc.
Installed Chromium152.0.7977.82 at $PREFIX/lib/chromium/chrome completed real
Playwright DOM input/click/readback from native Bun on Android. No account was used.

playwright_android.py corrects exactly three startup-cache predicates in the pinned
Playwright1.62.0 bundle. It verifies before/after hashes, owns only a TMPDIR checkout,
and does not spoof process.platform or download a substitute browser. Bridge CLI
startup and DOM interaction pass after the correction.

Model/tool acceptance remains open. Use a new private experimental browser profile;
do not import an existing browser or Codex account. Keep authentication local and
out of Git/logs. Do not run the candidate's ordinary setup: it installs services and
rewrites Codex routing. Exercise its server with owned configuration and a disposable
signed Core home first. Discover existing tunnel/key file references without printing
values; do not replace a live Tdev tunnel. Browser-only mode cannot prove tool use.

The earlier fixtures remain useful protocol evidence, not real model acceptance.
Require a real reply, two actual owned tool calls, cancellation, compaction and resume
with protected-state proof before considering integration. Do not use the candidate's
DEV synthetic-tool driver as evidence of these capabilities.

Temporary source/build roots belong to TMPDIR/XDG cache. Browser authentication and
other retained experimental data belong to a private XDG data directory with explicit
ownership. No production launcher, optional Manager dependency or permanent service
is introduced by this branch.

`termux_browser_probe.mjs` is the account-free native Android DOM proof. Supply an
owned candidate, absolute browser binary and private disposable root; set HOME,
CODEX_HOME and TMPDIR to its h/codex/t subdirectories. It validates the pinned
Playwright correction, browser hash, input/click/readback and closed-page isolation.
`termux_viewer.ts` serves official noVNC assets on loopback with a random per-path
capability and exact WebSocket origin check. RFB stays on a private Unix socket;
X11 requires a private MIT-MAGIC-COOKIE. Viewer state must remain mode600. This
is a temporary login aid, not a new permanent service or an exposed VNC listener.

Affected native browser-login tests pass4/9 assertions and full TypeScript checks
pass. Owned protocol/HTTP tests pass26; these are separate from model acceptance.
The bridge patch includes the affected legacy-tool regression expectation as well
as source changes. Experimental code is not admitted to the production launcher.

Open the temporary viewer with `python3 open_viewer.py OWNED_GUI_ROOT` as a
notification action. The existing termux-am-socket binary requires the complete
command as one argument. This helper records only return metadata; capability
URLs and browser authentication stay private. A separate-argv trial failed before
Activity launch and was corrected after its actual one-argument URI gate passed.

The original relative WebSocket path duplicated the capability directory in real
noVNC. `open_viewer.py` now supplies an absolute path. `viewer_browser_probe.mjs`
loads the URL from that same builder in actual native Chromium and requires
noVNC's connected state, the exact WebSocket path and a screen canvas. A raw
WebSocket greeting alone did not cover this browser URL-resolution defect.
No screenshots, raw canvas pixels or authentication are recorded by the proof.
Rendering is checked through aggregate non-black coverage and color counts only.

The temporary viewer adds explicit mobile keyboard/fullscreen controls without
editing the pinned noVNC source. Remote desktop resizing replaces scaled tiny
text; actual Chromium with a360px touch viewport proves a360x680 desktop and
focus on noVNC's virtual keyboard element. The same actual client must render
non-black desktop content, not only enter noVNC's connected state. This proves browser focus, not the
physical Android IME; that observation remains user-owned. The dedicated login
window was maximized only after its X11 PID, live image, start and owned display
were checked. wmctrl/xwininfo are ordinary X11 inspection utilities, not services.
Chromium remains a Termux-UID process with --no-sandbox; a private login profile
is data separation, not OS sandbox isolation. Android browser login/autofill/
passkeys do not automatically transfer into the remote Chromium profile.

Physical mobile acceptance is still open: the user observed a black screen after
the first mobile adjustment despite native client rendering success. The temporary
viewer now reports only connection/dimension/rendering aggregates to a same-origin,
capability-protected endpoint; arbitrary strings are rejected and no input, page
text, screenshots, credentials or URLs are retained. This is diagnostic evidence,
not a claim that the Android display/IME has been repaired.
