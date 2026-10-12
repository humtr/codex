# Termux ChatGPT Web experiment

This separate branch follows repository AGENTS.md. Production configuration,
profiles, conversations and services remain authoritative and unchanged.

Selected bridge: miuuyy/codex-chatgpt-web at
7cc8b8d8aadbbf729bd2bd43ca47fa15ea9965c0, version6.1.8. The prior
92a356f/6.1.5 baseline remains separate with its qualified native runtime. Frozen Bun dependencies
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

The temporary viewer uses a40px single-row toolbar: a manual keyboard button,
a compact automatic-keyboard switch, scroll/drag icon, Esc and fullscreen icon. Manual open/close does
not change the automatic preference; changing that preference does not open/close
the keyboard. Automatic keyboard is off by default; thev2 browser-local preference deliberately
ignores the older default-on setting. Opt-in auto remains separate from manual
visibility, and any gesture movement/non-tap classification cancels activation.
The controls move above the Android keyboard using visualViewport geometry.
Original noVNC assets stay pinned and unchanged; viewer-owned JS/CSS have explicit
capability-protected routes and contain no account information.

Native actual-client proof checks first/repeated touch, stale-focus recovery, three
RFB click/key DOM readbacks, independent manual/automatic state and toolbar bounds.
It also waits beyond noVNC's2-second startup-menu close: the previous version lost
input focus there, reproduced in the account-free fixture. Opening the keyboard
now cancels that pending menu action, closes its sidebar before input focus and
uses preventScroll. The live login display is never tapped by this fixture.
`viewer_touch_probe.py BUN CANDIDATE NOVNC_ASSETS` creates private Xvnc/Chromium
instances and stops its temporary servers after the proof. Its toolbar-only image
is from that account-free fixture; live login screenshots/raw pixels are never
retained. The native proof shows a40px toolbar with no control overflow at360px.

Mobile connection, ChatGPT display and matched horizontal width were confirmed
by the user. The ordinary persistent browser also opened without Cloudflare. Repeated IME
open/close worked after the former first-focus loss; the compact independent
controls and startup-focus correction still need actual phone confirmation.
Client diagnostics retain only connection/geometry/rendering aggregates, reject
arbitrary strings and store no input, page text, cookies, screenshots or URLs.
DOM focus in the automated client is separate from physical Android IME evidence.
Scrolling, mouse clicks and multiple touches do not activate automatic focus. A
pixel viewer cannot identify remote form fields, so ordinary non-form taps can
also open IME while automatic mode is on; the switch can disable that behavior.

Chromium remains a Termux-UID process with --no-sandbox. A private login profile
separates data and is not an OS sandbox. Android browser login/autofill/passkeys do
not transfer into the remote Chromium profile. Browser/account/model/tool-loop
acceptance remains open; normal production routing is unchanged.

Single-finger dragging defaults to scroll: viewer-owned capture handlers translate
noVNC drag gestures into wheel events at their fixed origin, leaving taps, pinch
and two-finger scrolling in the pinned implementation. The arrow button switches
to ordinary mouse dragging for selection. Native actual RFB proof verifies both
scroll gestures change the owned page scroll position and mouse mode retains a
real left-button drag without emitting scroll wheel input. Esc travels over RFB and closes a real
HTML modal in the account-free browser fixture. The live user's native Save dialog
was independently identified by X11 ownership and closed with WM_DELETE; spawning
that exact Save dialog in the automated fixture did not qualify, so native Save
cancellation by the new Esc button is still a separate physical observation.

The user confirmed the authenticated composer. Normal closure of only the owned
login browser completed storage capture and verification on2026-10-12. Private
storage-state and verification marker are mode600; the temporary login profile
was removed by the bridge only after successful verification. Saved authentication
reopens the same independent experimental browser. No account data is published.
The bridge's account detector currently reports sol/extraHigh/pro unavailable;
model capability and real reply/tool-loop acceptance remain unqualified.

A real phone scroll trial exposed a separate presentation defect: the newly opened
saved-account browser was508x345 while the viewer desktop was480x785. Wheel input
below the window reached Openbox's desktop-scroll binding and changed desktops.
`fit_browser.py` now maximizes and selects only a normal Chromium window whose
PID/start/image, private HOME/DISPLAY and process ancestry match the saved viewer
helper. It never selects dialogs or unrelated X11 clients. `open_viewer.py` fits
that window before opening a saved-account viewer. The native regression shrinks
the browser to508x345, verifies stale-owner refusal, maximizes it, resizes the
viewer to480x825 and requires a lower-area gesture to scroll the real page while
the X11 desktop remains unchanged. No global Openbox binding is removed to hide
the defect. Actual phone follow-up remains pending.

A subsequent phone trial found Chromium's500px minimum normal-window hint still
cropped a480px display. The owned fitter now clamps only that browser's minimum
size hints to the display and uses its fullscreen window state. Actual X11 window
geometry equals360x700 and480x785 in the resize regression; horizontal swiping
changes an owned wide page's scrollX, and both directions preserve the desktop.
The fitter is idempotent for an already-fullscreen browser and sets narrow-window
minimum hints once. A startup supervisor then only waits for browser exit; it
never polls/refits. Openbox handles fullscreen size changes. Browser ownership
records use the actual Chromium PID/start/image/private environment, not a helper.

Cloudflare repeated human verification in the saved-state automated browser is
an unresolved real-service gate. Cloudflare's supported-browser documentation
explicitly excludes Playwright for production challenges:
https://developers.cloudflare.com/cloudflare-challenges/reference/supported-browsers/
For manual troubleshooting, `manual_browser.py GUI CANDIDATE BUN STATE` imports
account login once into a private persistent profile, closes the initializer and
runs ordinary Chromium with no automation/debugging flags or attached driver.
It excludes challenge-clearance cookies from the import, solves no challenge,
spoofs no browser properties, and never exposes authentication. Existing account
state/verification stays intact. Profile/marker modes are700/600. This manual
view is not model/tool-loop acceptance or an anti-bot workaround for the bridge.
Manual verification may still need the user, and may still fail. Do not add stealth,
CAPTCHA solvers or claim the automated provider works based on this viewer.

## Current real-service frontier

The ordinary persistent browser's width and absence of Cloudflare were accepted
by the user. Reopening that same profile with Playwright is not accepted: a bounded
first attempt reached a composer but did not prove a reply; subsequent diagnostics
returned document403/200 and a challenge title, with no composer or model response.
No challenge was clicked or solved. The ordinary browser was restored normally.
Do not mistake manual browser access for automatic provider acceptance.
`persistent_model_probe.mjs CANDIDATE GUI DATA_ROOT` is the bounded reproducer;
it refuses a still-running qualified browser owner before touching the profile.
Only aggregate navigation/response metadata is retained; prompt/reply bodies,
account contents, cookies and challenge tokens are never printed or saved.

The6.1.8 input uses the same frozen bun.lock and qualified Playwright correction.
The prior Termux/wire patch applies cleanly; chatgpt-web.patch is now regenerated
against exact7cc8b8d. Six affected suites initially passed134, skipped one Windows
case and failed four authority tests because the calling Codex's CODEX_SQLITE_HOME
was inherited. Removing only that variable for the owned test process makes all
four focused cases pass; full TypeScript passes. Keep the inherited-environment
failure visible, and isolate subsequent tests rather than relaxing authority.
No6.1.8 real-model acceptance is claimed. Its ordinary managed-browser creation
still launches a fresh automated browser/context, so it does not fix the403 gate.
`mcp_catalog_probe.mjs CANDIDATE OWNED_ROOT` connects its actual native stdio child,
checks six unique object-shaped tool schemas and closes the child. Both6.1.5 and
6.1.8 pass this transport proof; there are zero real model/tool executions.

The existing, stopped surface-probe Tunnel was temporarily connected through an
independent alias codex-web-termux-trial and private TMPDIR profile, using its
existing runtime-key file. Native tunnel-client0.0.14 reports running/ready/healthy;
operating Tdev configuration and key hashes are unchanged. This is temporary
trial runtime state with no boot service; source/binary inputs remain outside prj.
It must be stopped/removed from local runtime metadata when the trial finishes.
The user confirmed creation and installation of Codex Native2 using the prepared
Tunnel. This is physical setup confirmation, not model/tool-loop acceptance. The official workflow is ChatGPT Plugins → plus → Add custom MCP
server → Connection: Tunnel; the app creator must have Tunnels Read+Use and the
Tunnel must belong to that ChatGPT workspace:
https://developers.openai.com/api/docs/guides/secure-mcp-tunnels?site_locale=en
The candidate requires a separate safe connector for Zero Risk/manual turns;
it is not silently interchangeable with this native automatic contract. Automatic
reply, two model-initiated tools, cancellation, compaction and resume remain open.

The user reported unusable scrolling after keyboard reopening/reflow, despite the
prior headless gesture proof. Live Android diagnostics showed desktop785→933px
when IME changed innerHeight/visualViewport. The owned layout poller was stopped
and removed without signalling Chromium; the current browser/form is preserved.
Remote desktop height is now held across keyboard/address-bar changes, updating
only for width/orientation or explicit fullscreen change. Manual keyboard defaults
off; opt-in automatic focus is cancelled by movement at any point and by noVNC's
non-tap classification, including an out-and-back swipe ending near its origin.
Esc sends its key over RFB without focusing a hidden input. Native account-free
regression models the observed Android layout with real innerHeight changes and a
simulated visualViewport. It requires constant canvas/X11 height and no keyboard
focus after an out-and-back scroll with stale input focus. This regression is layout simulation, separate from physical Android IME proof.
The user subsequently reported the viewer became usable after this exact change;
that physical acceptance is recorded independently in the private proof. Existing gesture/input/Esc/width gates pass.
The user also reported “app setup was rate limited”. No app creation retry or rate
limit bypass was made; the user subsequently completed ordinary plugin installation after the viewer
fix and real native tunnel connection. The temporary Tunnel itself remains ready.

Current physical baseline: scroll usability was accepted after default-manual
keyboard, fixed desktop height and layout-poller removal. A later phone report
reopened the horizontal-width gate. Aggregate viewer records include a zero-sized
Android viewport and framebuffer; these invalid/background layouts must not
resize the shared desktop. Preserve the last valid height, resize only while the
viewer is visible with positive dimensions, and scale the framebuffer to the
valid viewport while the remote resize completes. Keyboard-only height changes
remain excluded. Native account-free regression passes hidden zero-layout
preservation, a second viewer changing the shared desktop to640px while the
480px viewer remains uncropped, visibility restoration to480px and the existing
input/scroll/IME gates. Renewed phone acceptance remains pending. The same ordinary Chromium,
login and form remain alive; trial tunnel status is ready/healthy and operating
Tdev config/key hashes remain unchanged. Plugin installation is now confirmed by the user. Automated browser403 and
model/tool-loop acceptance remain unresolved service gates.

The first real tunnel trial used a standalone Linux/arm64 CGO-disabled CLI.
Its startup health was green while outbound polling failed DNS against localhost53:
Android has no /etc/resolv.conf. Local ready/healthy is not connection acceptance.
Tdev's separately owned bin/tunnel-client is already Android/CGO and stays unchanged.
The trial now uses exact OpenAI source0f870e50a973fa820d4c409000059e181e8d242b,
compiled on this phone with Go1.27.0, GOOS=android, GOARCH=arm64 and CGO_ENABLED=1.
The build uses -trimpath, -buildvcs=false and the upstream pkg/version.GitSHA flag.
Build/dependency caches live under XDG cache; source is an owned TMPDIR checkout.
Native net.LookupHost succeeded and actual OpenAI long-poll successes are measured
through the owned loopback metrics endpoint. Neither system resolver files nor
Tdev runtime/config/key bytes were changed. The original runtime key file reference
and tunnel ID are reused; no admin CRUD, key rotation or new tunnel was performed.

The same owned Native2 stdio target has env -i, a private HOME and an absent
TurnBroker socket. Its six-tool catalog is qualified, but no real user tool can
execute until the broker exists. The user confirmed plugin creation/installation. Actual model/tool calls remain
unqualified, and the TurnBroker is not yet connected. The normal Chromium profile stays open and unchanged. Ordinary Linux CLI
status still reports control-plane snapshot unknown; actual poll metrics, not that
startup label, prove the outbound connection. The Android artifact is retained in
~/.cache/codex-web-experiment/tunnel-client/<SOURCE_SHA>/tunnel-client; the running
alias/source/profile are a temporary experiment, not a durable installed service.
Private aggregate proof: ~/.cache/codex/acceptance/web-native-tunnel.json.
