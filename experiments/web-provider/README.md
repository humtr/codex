# Termux ChatGPT Web experiment

This separate branch follows repository AGENTS.md. Production configuration,
profiles, conversations and services remain authoritative. The desktop-provider
trial adds only an explicitly selected upstream configuration profile; base
configuration and the signed Core/Manager installation remain unchanged.

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

Browser/tunnel model acceptance remains open. The separate signed-desktop transport
now passes actual ChatGPT/native-Termux tool proof below. Use a new private experimental browser profile;
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

Current physical baseline: the user rejecteddca030e on the phone: right-edge
cropping persisted and the screen appeared black, despite green account-free
zero-layout and multi-viewer regressions. Those tests are not phone acceptance.
Live mobile controls and the corresponding fixture are restored byte-for-byte
from user-accepted023c263. Original ordinary Chromium is fitted to480x785 and its
PID/start/image, private profile and login are preserved. The successful reopen
notification uses that original viewer plus an explicit `--refresh` URL query,
so it loads the controls afresh without a manual reload. The user then confirmed
this restored connection works. Keep this accepted configuration while diagnosing
the separate automatic browser; do not reintroduce the rejected width experiment. The same ordinary Chromium,
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

The native Chromium host remains an experiment. It uses one owned persistent
profile, private DevTools pipes and a bearer-gated loopback relay. Private
PID/start/image/profile descriptors, one-client reservations and disconnect
cleanup are mandatory; no unauthenticated Chromium debug port, browser-property
override or challenge solver is used. Setup/doctor/installer admission is not
finished and this mode is not merged into the production runtime.

Current operating baseline is the user-confirmed ordinary browser and023c263
mobile controls, restored ine72f1e8. Use `open_viewer.py GUI --refresh` to reopen it
with a fresh URL query without a manual reload. Preserve the original profile,
login and form. The rejecteddca030e width behavior is not active.

`fresh_login_probe.py` reads only the experimental Chrome cookie DB through a
SQLite backup into a disposable profile. Challenge cookies and other domains
are excluded. The old saved seed had24 account cookies while the current profile
had27; the current snapshot reaches an authenticated unique composer. These
snapshots and owned processes are removed after each trial.

`native_session_probe.mjs CANDIDATE DESCRIPTOR PROOF --retain-page` prepares one empty Temporary Chat and
hands off its exact CDP target ID through the owned descriptor. The worker
consumes that ID once; stale targets, existing user turns, nonempty drafts and
ambiguous editors are refused. It avoids closing the authenticated first page
and repeating navigation during inspection and actual task startup.

`live_core_probe.py` uses the actual signed stable49 Core, actual6.1.8 Responses
server and TurnBroker in disposable roots. A public Web route is required and
native API pass-through is refused. It reaches authenticated surface/model/effort,
exact Native2 selection, the complete prompt/files and Send. The conversation
POST then returns403 with `cf-mitigated: challenge`, and the actual adapter's
`chatgpt_security_check` guard rejects the turn. That is the model submission,
not a failure of the existing viewer or Tunnel connection. Real model reply,
two model-initiated tools, cancellation, compact and resume remain unqualified.

`profile_reuse_probe.py` confirms the same result using the unchanged experimental
profile in place. It normally stops only the owner-verified dedicated Chrome,
creates a private recovery checkpoint, preserves tabs and always restores ordinary
Chrome/its owner marker in finally. Codex/Tdev/other profiles remain running.
The human-check option waits at most120 seconds, sends its navigation notification
when ready, makes no model request while waiting, then restores ordinary Chrome
and removes the stale alert. The indefinite hold was rejected by automatic
approval review and was not executed; the bounded variant was allowed. The user clicked verification but reported that it returns to Verify repeatedly.
The native-host human gate has failed; it must not be described as merely waiting
for user input or treated as provider admission. Ordinary Chrome is restored.
Do not schedule another identical verification view without a new technical basis.

Native TypeScript passes. The six affected suites pass177/0 failures and1005
assertions, including five actual Android Chromium DOM cases. Account-free pipe
proof passes descriptor/auth/origin/concurrent-owner refusal, prepared-page
consumption/readback, stale target/draft/user-turn refusal, persistent context,
reconnect plus loopback navigation, and owned cleanup. These are transport and
renderer gates, not a substitute for a real ChatGPT model/tool turn. Private
aggregates: web-current-login.json, web-same-profile.json, web-human-check.json
and web-warm-page.json under ~/.cache/codex/acceptance; credentials stay local.

Cloudflare defines the challenge header and does not support automated frameworks
for solving production challenges:
https://developers.cloudflare.com/cloudflare-challenges/challenge-types/challenge-pages/detect-response/
https://developers.cloudflare.com/cloudflare-challenges/reference/supported-browsers/

`native_chromium_probe.py BUN CANDIDATE --compatibility` performs an account-free
read-only run on Cloudflare's public diagnostic site. It loads with document200,
JavaScript/cookies/secure context/WebCrypto are available, but WebGL1/2 are absent
and the real browser reports automation. No IP/fingerprint/body/credentials are
output, and no production challenge is acted on. Zero recognized diagnostic
status cells are not a passed compatibility gate. DNS and the public validation
script are reachable (HTTP200); this does not prove every in-browser challenge
request succeeds. Automated-browser incompatibility is supported by the repeated
production challenge, native automation state and Cloudflare documentation; the
specific triggering signal and any effect of missing WebGL remain unproven.

The154 package was downloaded into TMPDIR and verified against the signed apt
metadata SHA256; the installed152 browser was not upgraded. Equivalent diagnostic
launches on152/154 both expose WebGL only with Playwright's unsafe SwiftShader
fallback enabled; excluding it leaves WebGL1/2 unavailable on both versions.
The version comparison therefore does not prove an upgrade fixes the challenge.
The new binary/archive were removed after the comparison. This unsafe graphics
fallback is not enabled in the ordinary/native operating browser.
The user-confirmed manual verification loop fails the current native-host route;
there is no pending human-input explanation or production provider admission.
Specific bot signal causality remains unknown. Keep the connected ordinary
browser and existing Native2 Tunnel, preserve the experiment in Git, and require
new technical evidence before another production challenge attempt.

## Official alternatives after the failed manual verification

The official Linux preview now has a ChatGPT/Codex desktop package for ARM64:
https://learn.chatgpt.com/docs/linux/linux-app
It supports Debian13 and selected Ubuntu/Fedora/Arch desktops; Android/Termux is
not a supported desktop distribution. This is separate from the already working
native Termux CLI and is not a replacement production generation.

The 26.1007.21434 ARM64 package was downloaded into TMPDIR. The official repository
key fingerprint 3BFA0E4AE8B8CC16A2D9BA684A3B4A566C4660E4 matched the published
installer; gpgv verified InRelease, its SHA256 authenticated the arm64 Packages
index, and the complete 475083462-byte package matched
55fae1e3fb1d6069a90620e9e70737dcf8bf2255604d157e282cc6748a1baf49.
A separate Debian13-slim rootfs used official PRoot-Distro source 5a427a1 with an
owned prefix/HOME and --isolated bindings. Apt dependency installation and the
real app --version passed. Only guest packages/configuration changed; a guest
policy-rc.d refused service starts. Public CA certificates were installed in the
guest; the host resolver and existing runtimes were not modified.

Bounded account-free Xvnc/Openbox trials use private Unix sockets/Xauthority,
no copied login and the actual packaged executable. The app created a full-sized
ChatGPT window and initialized its bundled 0.162.0-alpha.17.2 app-server over
stdio. Initial snapshots showed a loading/blank shell. After a longer bounded wait
and --disable-dev-shm-usage, the actual sign-in page rendered with both login
choices. Rootfs CA certificates were also installed; the decisive change is not
isolated. This is visually inspected login rendering, with no user login or
model turn. A 10x10 utility window is not a render gate. Electron's sandbox
was disabled only for this owned Android/PRoot trial, where Linux namespaces are
unavailable; that does not establish desktop security or production support.
All owned launchers are stopped after observation. The unauthenticated guest
rootfs and both temporary source checkouts were removed after inspection. The
verified public package and account-free render fixture stay in XDG cache for
reuse; aggregate proof is ~/.cache/codex/acceptance/web-linux-desktop.json.
The existing ordinary browser and Native2 tunnel are separate and preserved.

A more direct official option for the Core/provider objective is ChatGPT plan
usage through Sign in with ChatGPT, without browser model submission:
https://developers.openai.com/siwc/token-sharing-open-source
https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
Local open-source apps register a dynamic OAuth client, persist one host ID and
ask the user for chatgpt.tokens.use.direct consent. No partner key/client secret
is required. This is a new grant; existing Codex or browser credentials are not
substitutes. Use the account's actual model catalog, the public Responses API,
store=false and stream=true, and require response.completed/turn completed.
Function/custom tools are supported in namespaces and Core can execute its own
local tools; hosted MCP, tool_search, image generation, Code Interpreter and
native computer use are unavailable on this route. It does not establish parity
with every ChatGPT Web feature or the desired account/model entitlement.

Official devkit source 0a36fefe was inspected, not installed or authenticated.
Its local runtime requires an OS-backed credential encryption provider. Do not
weaken that guard, borrow first-party OAuth client IDs or silently replace any
active profile. The next qualification is a separate explicit user grant,
protected token storage, a completed real account/model turn and an actual Core
tool loop. No current trial passes those gates, and no production provider,
auto-start service or global launcher is added.

## Two authorized independent tracks

`linux_desktop_trial.py prepare/start/serve` reuses the accepted signed ARM64 package
in XDG cache, with a pinned PRoot-Distro source checkout in TMPDIR. The candidate
OS lives under the version's cache/trial-prefix. Its sole permanent account,
configuration and conversation home is
~/.local/share/codex-web-experiment/linux-desktop/home, bound as guest /root.
No host projects, existing profiles or real HOME are bound into the guest.
A private Xvnc/noVNC screen retains the accepted mobile controls. A candidate-only
tint2 panel reserves 42 pixels at the bottom; the app maximizes in the remaining
480x743 work area instead of covering the panel. It provides a ChatGPT task button
and a Browser button that opens Chromium in the same Linux desktop. Chromium
is installed only in the candidate Debian rootfs and uses /root/browser-profile,
separate from the app and all existing Android/native browser accounts. It opens
about:blank once; repeated clicks raise the same owned-profile X11 window without
creating tabs or requesting an Android activity. A 0.9 device scale allows the
browser's minimum width to fit the mobile desktop. The helper receives the
explicit host controller marker, validates PID/start/executable, and atomically
queues a show request. The controller owns browser creation and cleanup.
The separate guest authentication opener still forwards only
recognized OpenAI authorization URLs to the normal Android browser. It never
records the URL, code or token. Candidate HTTP/HTTPS MIME defaults select the
owned Android handoff desktop handler, while preserving the Codex callback and
other associations. Without this override the minimalist guest selects
chatgpt.desktop for HTTPS and its sign-in button routes back into itself. An
actual guest `gio open` dispatch reached the handler with an unrecognized,
account-free target; it was refused without opening an external page. Real
sign-in delivery remains a separate user gate. Subsequent signed-app source
inspection found that the actual first-party URL constructor uses
`https://auth.openai.com/oauth/authorize`. The app's actual default button first
wraps that URL in `https://chatgpt.com/codex/desktop-auth?authorize_url=...`.
An owned X11 button click reached the guest handler with that outer URL; its
query values were kept out of diagnostic output and the private diagnostic copy
was removed. Both the direct and default wrapped paths are accepted now. The
wrapper must contain exactly one authorization URL targeting the allowed OpenAI
path; another wrapper, foreign host or token endpoint refuses. Other origins,
credentials/token-bearing queries, fragments, nonstandard ports and raw
whitespace also refuse. No authorization URL is rewritten. The validated pending
URL lives only in the temporary 0700 desktop root as a 0600 file. Authentication
handoff is independent of the panel Browser button; ordinary browser launch
never reopens authentication or leaves the Linux desktop. This fixes local
refusals, not the organization's policy or proof of completed login. Candidate configuration folders
created by the app with mode 0755 are tightened to 0700 after verifying ownership
and refusing symlinks; existing configuration content is preserved.

The fresh `Linux 앱 로그인` notification opens this separate screen. Both trial
notifications resolve the current endpoint at tap time and validate the owner's
PID, start time and executable; stale process records refuse. A controller
restart therefore does not leave the notification pointing at an obsolete
port/capability. No boot service or global launcher is installed. `start` runs
the same controller detached from the tool command's lifetime, with private
controller.log and stdin closed. Default `--seconds 0` keeps this authorized
interactive candidate alive until the app closes or its owned controller stops;
a positive duration up to 3600 still bounds disposable trials. Clean shutdown
removes notifications and all owned app, browser, viewer and display processes
while retaining candidate account data. SIGTERM also enters that cleanup before exit; an
older controller did not do this, and its four owned orphan process groups were
identified by the private display/root and stopped. Restart the trial before
sending a fresh alert.

`siwc_trial.mjs` is a separate native Android public-client qualification. It
uses the official devkit's unmodified low-level OAuth, model and SSE modules;
the devkit's encrypted high-level ConnectionStore is not used or weakened.
Termux:API's available Keystore interface supports signatures, not credential
encryption. This trial instead implements the official sign-in guide's atomic,
owner-only file storage under ~/.local/share/codex-web-experiment/siwc (directory
0700, records 0600). Tokens remain plaintext inside Android's app-private storage;
this is file protection, not an OS-backed encryption claim or a new key service.
A single lease serializes this qualification process and rotating token writes.
The stable host ID, issued registration and verified account remain distinct;
pending refreshes are preserved but are not authorized for inference until the
vendor verifies them. Symlinks, nonprivate records and malformed connections
refuse. Existing Codex/browser accounts are never imported or modified.

The private loopback page for `공식 ChatGPT 구독 연결` has one Continue with
ChatGPT action. Only same-origin POST starts OAuth; a capability gates every
route. The normal Android browser displays OpenAI's own registration/consent
screen for codex-termux-trial. A successful callback validates PKCE/state and
OIDC signature/issuer/audience/nonce before saving credentials. Direct scope is
required; no API-key billing fallback exists. Model discovery uses the actual
account catalog, preferring gpt-6.1-sol when listed. A short exact reply must
complete before `siwc_core_probe.py` drives the signed installed Core's actual
app-server and shell tools in a disposable HOME/PREFIX/workspace. The bearer
credential is passed only to the Core process; its shell environment is explicit
and excludes the token. Only completed turn, successful shell execution and the
owned output marker establish a real tool-loop gate. No current user data is
submitted, and logs/status contain no tokens, callback codes or response bodies.

Native Node24 compilation and 27 vendor OAuth/refresh/SSE tests pass. Six
actual Android persistence/privacy/UI cases pass. The real signed stable49 Core's
account-free app-server proof passes two correlated shell tool calls and public
route request-shape checks, with all owned Core processes removed afterward.
That fixture is synthetic service proof, not real subscription inference. Two
additional Android filesystem/routing tests pass: idempotent MIME defaults
preserve other associations and harden owned candidate configuration, and a tap
uses the current port while refusing a reused PID/start record. The delivered
subscription page returns HTTP 200 and its inline script passes syntax checking.

The user's attempted external sign-in returned `3p_login_tenant_policy_denied`.
That reported state is preserved separately from verified credentials. The
ChatGPT workspace Apps screen does not control external SIWC identity access;
that policy belongs to the tenant Global admin's External access controls.
Official plan-usage eligibility currently documents Plus/Pro, so removing a
Business identity restriction alone would not establish plan entitlement. No
organization policy was changed and no other account was selected automatically.
The first-party Linux app is an independent qualification track. User sign-in
now completed, the Android handoff reports success, and candidate auth storage
contains ChatGPT credentials with a Business-family (`team`) claim. The claim is
an account label, not an independent live entitlement check. The actual bundled
Linux Core then completed an ephemeral real-service turn in an isolated guest
workspace: two successful shell calls wrote and read a correlated marker, and
the completed model reply matched it. `guest-core-proof.json` contains only
aggregate evidence; credentials stayed in the candidate home, with no existing
Termux profile imported. The temporary workspace/event logs and owned probe
process were removed. This establishes signed-app bundled Core inference/tool
execution, not desktop UI or all ChatGPT Work features. The agent sent a synthetic prompt through the actual native GUI and visually
verified its distinct uppercase reply. That confirms a GUI response; clipboard
readback failed and is explicitly recorded as unaccepted, rather than erasing
the visual proof. Desktop ChatGPT Work/tool parity remains unqualified. Recent app diagnostics still contain a primary
runtime manifest 404; do not infer its resolution from the Core proof. SIWC real
account/model/tool acceptance remains unpassed. Production remains stable49.


## Public Responses versus Business alternatives

The SIWC plan-usage qualification calls the public
`POST https://api.openai.com/v1/responses` with the consented OAuth token,
`store=false`, and `stream=true`. Its official Core provider uses the public
base URL and `wire_api=responses`; it does not authenticate with the ordinary
Codex profile. Current open-source eligibility documents Plus/Pro. The reported
Business tenant identity denial is real, but neither changing External access
nor substituting a Codex token establishes SIWC plan entitlement.

| Route | Contract | Business relevance | Qualification limit |
| --- | --- | --- | --- |
| SIWC plan usage | Public `/v1/responses`, user-granted OAuth scopes | Business eligibility not established; this account was rejected | Real model/tool gates unpassed |
| Platform API | Public `/v1/responses`, standard project API key | Independent API organization/access/billing, not included Business quota | No paid inference was initiated |
| Codex access token | First-party CLI/app-server workspace credential | Official Business/Enterprise support; installed CLI has `--with-access-token` | Codex automation, not a general Responses or full ChatGPT Web grant |
| Workspace Agents API | `api.chatgpt.com/v1/workspace_agents/{id}/trigger` | Business preview subject to workspace enablement/scopes and a published agent | Asynchronous; status polling exists, but final answer body cannot be retrieved |
| Linux first-party app | Official desktop sign-in and product capabilities | Independent from external SIWC denial | Android/PRoot is experimental; login, bundled Core tool loop and native GUI reply pass; Work parity unqualified |

A Workspace Agents bridge could start a published workflow and use an allowed
MCP result callback to an owned broker. That is an architectural candidate,
not an implemented or verified provider. Wrapping its trigger in a local
Responses-shaped endpoint would not create native Responses streaming, model
selection, cancellation or full tool-loop parity. The callback and agent's
permissions would first need end-to-end proof. A standard project API key uses
separate API charges; the existing Admin API key is for administration and is
not an inference credential. No credential was repurposed or organizational
setting changed during this investigation.

Sources consulted on 2026-10-13:
- [SIWC models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [SIWC Core configuration](https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server)
- [Open-source eligibility](https://developers.openai.com/siwc/quickstart)
- [Business Codex access tokens](https://learn.chatgpt.com/docs/enterprise/access-tokens)
- [Workspace trigger and result limitations](https://developers.openai.com/workspace-agents/trigger-runs)
- [Workspace Agents Business preview](https://developers.openai.com/cookbook/articles/chatgpt-agents-sales-meeting-prep)
- [API credential types](https://developers.openai.com/api/reference/overview)
- [Independent API usage pricing](https://learn.chatgpt.com/docs/auth)


The Plus trial now has explicit user authorization to use the user's separate
Plus account. SIWC requires user OpenAI sign-in and plan-usage consent, not a
new Admin Console/Platform key. A fresh Plus notification was delivered; real
consent/inference is still pending. Linux source/marker/routing regression tests
now total five, including the panel helper running under a different candidate
HOME and refusing a stale process owner, and owned browser window selection
excluding hidden helpers and other profiles. Actual native panel parsing, X11 window
and work-area reservation pass; the owned signed-out screenshot shows Browser,
ChatGPT and the clock without covering the app. A SIGTERM restart exited zero,
removed the old temporary desktop and marked the stopped controller before the
new launch. Both normal browser account state and production stable49 remain
unchanged.


The Linux Browser button now passes an actual native X11 click test: the first
click opens one candidate Chromium window and the next click raises that same
window. Its geometry is 480x743 and the private viewer returns HTTP 200. Browser
launch requests no Android activity; only the independent OpenAI login handoff
and notification viewer opener can leave the remote desktop. The controller
owns browser processes and repeated clicks do not launch another process.
Chromium's profile-specific WM_CLASS is inspected on the private display because
this environment's _NET_CLIENT_LIST can omit that client. Hidden 10x10 helper
windows and other profiles are excluded. Native package dependencies include
`tint2` and `xdotool`; candidate `prepare` includes Debian Chromium. No new boot
service, launcher in PREFIX or production profile is installed.

An older foreground controller vanished while its app/display children survived.
The stale owner marker refused notification taps, which explains why removing
the timeout alone did not prevent a missing screen. `start` now detaches the
same controller from the tool's session; its PID/start/image remains verifiable
across completed commands. Old orphaned candidate process groups were identified
by their private root/display and stopped; unused private screens were removed
only after checking that no live process referenced them. App/browser login
profiles stay in the candidate's permanent private home.

The current detached candidate also passed a fresh actual GUI reply: native input
sent a lowercase synthetic marker and the visible assistant reply matched the
required uppercase marker. The app task button returns to that response from
Chromium. Aggregate evidence is guest-core-proof.json and
browser-panel-qualification.json, never authentication or user conversation
contents. The previously observed clipboard-copy failure stays separately
unaccepted. Primary runtime manifest 404 diagnostics remain, although the
bundled Core and actual GUI reply complete; Work parity and production admission
remain separate gates. The candidate stays open until closed, and a fresh viewer
notification targets the current owner.

## Actual ChatGPT in native Termux Codex

The signed Linux app's **ChatGPT → Chat → Instant** surface now supplies an
opt-in local Responses provider. This is a UI transport to actual ChatGPT, not
a renamed bundled Codex backend, public OpenAI API inference, or SIWC credentials.
`chatgpt-desktop/instant` names the bridge's selected UI mode; it does not assert
an independently verified underlying model slug. The Linux app stays an
experimental Debian/PRoot host on this Android device.

The path is native installed Termux Codex → authenticated loopback
`/v1/responses` → signed app ChatGPT → native approved tools → ChatGPT follow-up.
`desktop_responses_server.mjs` reuses parser, wire names and SSE serialization
from the qualified 92a356f bridge checkout, without starting its browser adapter,
launcher, service installer or native-provider passthrough.
`desktop_response_transport.mjs` drives only an owned Temporary Chat through
private FD3/FD4 and the existing capability-authenticated CDP relay. It binds the
renderer identity, checks Instant and an empty composer, and correlates each
structured reply with a fresh request marker. Fresh setup waits until the old
correlation has disappeared from the Temporary Chat. Context acknowledgement and
work-request instructions are separate, so the model is not told to acknowledge
and execute in the same message. Large initial context is staged
in ordered acknowledged batches before the actual task: whole tool schemas,
lossless ordered instruction/message fragments and an inert context phase.
Native tool-search support avoids eagerly loading every configured plugin
schema. Small requests stay direct. Subsequent requests send only new native messages
and changed instructions/tool lists; changed historical prefixes are refused
instead of silently losing context. The default transport uses the app’s internal Chat submission; the serialized
UI fallback uses its enabled Send control. Core owns approval and actual tool
execution. Exact tool results and immutable call-ID replay checks stay in the
local adapter; no guest Linux tool impersonates native Termux execution.

The installed opt-in provider has one shared registration:
`~/.local/share/codex/core/config/profiles/chatgpt.config.toml`. Its command-backed
login helper bootstraps the existing app/adapter on demand. The qualified Core
projects a missing configuration-profile name into any owned account home;
existing local overrides win. Manager is not required for this path adaptation.
The installer accepts no `--home` for registration alone; explicit `--home PATH`
can register existing homes on older installed Cores. It converts only an exact
previous generated config into an alias, preserving edited local profiles.

```sh
python -B experiments/web-provider/install_desktop_provider.py
codex --profile chatgpt
```

`--profile chatgpt` remains the fallback. The separately qualified `/model`
overlay uses the same shared registration from an ordinary launch, keeping the
account home, durable thread, permissions and terminal. Neither route is a
Manager login profile or a change to the ordinary default model.

The current 0.161 client reads `chatgpt.config.toml`; the earlier inline
`[profiles.chatgpt]` form is rejected by this client. This upstream **configuration
profile** is separate from a Manager login account. Resolve the account normally,
then select the config. `/model` displays **ChatGPT · Instant** in this profile.
An ordinary `codex` launch keeps its normal provider and model. Profile creation
refuses an existing user-owned `chatgpt.config.toml` and validates the local
server PID/start/image, private capability file and loopback endpoint. Restarting
the server changes its capability/port; reinstall this owned config after
restart. Authentication stays command-backed and private; no API/Admin key,
first-party token extraction, production daemon or boot service is required.
Rollback removes only the verified shared registration and aliases owned by this
experiment; existing local overrides remain. Never
restore a whole base config over subsequent user edits.

Real signed native stable49 acceptance completed two separate local shell
operations, file readback, returned results and an exact final nonce. A stronger
run submitted a real desktop turn, interrupted before any tool, stopped the
first Core, resumed the same native thread in a new Core and completed the two
commands and final reply. The native TUI also opened `/model` and showed
the ChatGPT/Instant entry. `native_desktop_cli_probe.py` qualifies the actual
installed `codex --profile chatgpt exec` path separately from a disposable Core. Browser-only fixtures are not these proofs. The actual installed command also
completed a 90,000-character staged native context with two real commands and an
exact final reply. Capability-free/browser-origin requests returned 401, and a
second native thread during that real turn returned 409 without submitting
another model request. Protocol checks cover staging order/lossless fragments,
whole schemas, virtualized code blocks, immutable results and bounded retries.
Real send-failure Retry acceptance remains distinct from these unit checks.
`native_desktop_cli_probe.py --tool-search` separately proves real native catalog
lookup and schema delivery without executing any discovered tool or changing
user files. Qualification uses explicit danger-full-access/approval-never only
in its disposable workspace; ordinary account permission preferences remain
unchanged.

Clear UI send failures use the app's own Retry control with one- then two-second
backoff, at most two extra attempts. The original packet/correlation is retained;
no extra prompt is inserted. Cancellation stops retries, and an already arrived
correlated reply is consumed before any retry. Exhaustion, security checks,
unsupported context, stale owners and invalid tool replies are explicit errors.
A timeout with uncertain delivery does not trigger blind resubmission.

Remaining admission limits: one physical ChatGPT lane is serialized; a second
active thread gets a conflict rather than shared results. Text and advertised
function/custom tools are supported. Native tool-search wire mapping is enabled
so existing plugins are deferred by upstream. A real full-permission native
catalog lookup now returns matched results, loads schemas and completes the
exact final reply. The CLI omits search items, so qualification observes the
actual Responses call/result rather than a display item. Actual execution of a
discovered read-only GitHub repository tool also passes the installed CLI gate.
Images/files/audio and remote encrypted compaction are refused. Native
Responses/memento text compaction now passes the real Core gate below. Cancellation retires the interrupted
ChatGPT lane; native resume replays the native history into a fresh owned
Temporary Chat. Automatic pre-turn/mid-turn compaction and cancelled/failed
compaction now pass the device gates below. Hours-long load, parallel sessions,
extended failure recovery and reboot/update lifecycle still need acceptance
before production admission. The user
deferred physical width correction. Plus SIWC/public Responses remains a separate
unauthenticated trial; desktop success establishes no public API entitlement.

Local compaction follows native Core's Responses/memento contract. The bridge
requests a real ChatGPT summary with no executable tools; native Core owns
replacement history and persistence. A pending summary is bound to the same
native thread. Only its exact upstream-prefixed replay authorizes retiring the
old bridge-issued call/result checks and opening fresh ChatGPT context. Arbitrary
history replacement, a different thread or a forged summary does not authorize
retirement. Remote encrypted compaction formats remain separate. Qualification
must preserve a secret learned only from an actual native tool, compact, continue
with fresh native calls and stop/resume Core without revealing that secret again.

The real `siwc_core_probe.py --compact` gate passed on signed stable49: learn a
random secret only through a shell result, write it in a second call, remove
only those generated nonce fixtures, obtain/persist a real ChatGPT summary,
continue with two new native calls from that checkpoint, then stop/resume Core
and recall the value without a new tool. The secret was never repeated in a
user task. Four successful native shell calls, the matched Core compaction
item, exact offered-summary replay and persistent restart recall all passed.
This qualifies manual local compaction. The automatic pre-turn and mid-turn
gates also pass with matching native items and four native shell calls each.
Cancellation after actual Chat submission and rejected local relay credentials
commit no partial summary and retain the tool-only secret across Core restart.
A subsequent successful summary, two new shell calls and another restart pass
in both fault cases. No forced context trimming is substituted for compaction.

Quota classification is separate from functional acceptance. This provider uses
the signed app's Chat/Instant surface and routes native primary inference only
to the local bridge. Expected metering is the logged-in app account's ordinary
Chat allowance. A bounded device comparison now completed one installed
`codex --profile chatgpt exec` Chat/Instant reply and one ordinary native
`gpt-6.1-sol` reply using the signed app's Business login. The reference Codex
session uses a different Plus account, so its counters were excluded. The native
control used an admitted signed Core in disposable state with a non-refreshing
authentication copy; the original login file remained unchanged. Read-only
backend usage and token-profile measurements before, immediately after and
after delayed reads all retained 0% primary/weekly usage and unchanged token
counters. The positive Codex control also caused no visible increment, so this
small comparison cannot prove billing classification or zero Codex deduction.
App-server rounds usage percentages to integers; the raw server response was
also integer-valued in this observation. No quota was burned to force a visible
increment. Private aggregate proof is
`~/.local/share/codex-web-experiment/native-provider/quota-comparison-proof.json`.

The independent official SIWC plan route consumes the Codex/Work allowance, and
Work shares usage with Codex. Sources: [SIWC usage](https://learn.chatgpt.com/docs/sign-in-with-chatgpt)
and [Work/Codex pricing](https://learn.chatgpt.com/docs/pricing).

The wire API version is **v1**, `POST /v1/responses`; the private local adapter
implements the qualified text/tools/streaming subset, not every public Responses
feature. The runtime parser/SSE dependency is bridge **6.1.5 at 92a356f**, the
installed native CLI is **0.161.0**, and the signed desktop app is
**26.1007.21434**. These software versions are separate from the API path version.

Automatic compaction requires local context accounting. The desktop app does not
expose authoritative token usage. The adapter supplies an explicitly estimated
UTF-8 context/output size to the native token budget and marks terminal Responses
metadata `termux_usage_source=estimated_context`. These values size local context;
they do not measure server tokens, quota deduction or billed usage. Zero is not
an acceptable substitute for missing usage. Native Core remains the authority
for triggering, committing and persisting automatic compaction. Device acceptance
must show an actual automatic contextCompaction lifecycle and preserved tool-only
state; a manually requested summary alone does not qualify automatic compaction.

The device qualification driver supports `--auto-compact` for native pre-turn
compaction, `--mid-turn-compact` for rollover between actual native shell calls,
`--interrupt-compact` for cancellation after an observed real Chat submission,
and `--failed-compact` for a locally rejected capability followed by recovery.
All use admitted signed stable49 Core in disposable roots. Faulted compaction
must commit no summary and must preserve the tool-only secret across restart;
recovered compaction then completes, performs two native calls and survives
another restart. Lower automatic thresholds and generated reference text belong
only to these disposable tests, not the user's configured sessions.

`native_desktop_cli_probe.py --tool-action` exercises the installed upstream
ChatGPT configuration profile with actual catalog lookup, execution of the
returned read-only GitHub metadata tool for humtr/codex, and native result/final
reply correlation. This is real connector execution; earlier search-only proof
remains separately labelled. Qualification does not authorize unrelated writes
or messages and does not replace the installed native tool execution engine.

Continuation architecture and scenario costs

Core owns the canonical transcript, actual tool results and committed summary.
The adapter holds only one disposable cursor for its physically resident context.
Neither the configuration profile nor a future /model menu owns another history.
Route identity comprises provider, effective account/workspace, model and the
actual supported mode/effort. Connection generation and checkpoint epoch are
separate: a live model name alone does not prove its previous context survived.

| Event | Native record | Adapter transmission | Cache status |
| --- | --- | --- | --- |
| Same route, live owned context, append-only history | Retain | New messages only; changed settings explicit | Unknown unless provider reports a hit |
| Model or provider changes | Retain compatible history | Replay current Core projection | Reuse not assumed |
| Effort changes | Retain | Proven in-context update if supported; otherwise replay | Settings may affect matching |
| Instructions/tools change | Retain | Explicit replacements plus new messages | Changed prefix may miss |
| Account/workspace changes | Require explicit authorized target | Fresh isolated replay | Never assume cross-account reuse |
| App/relay generation changes | Retain | Fresh replay even if model name is identical | Unknown |
| Adapter/Core restarts | Resume persisted Core thread | Replay after validating owned endpoint | Unknown |
| KV entry expires while context is still resident | Retain | Continue normal append | Miss does not affect correctness |
| Native compaction commits | Retain committed checkpoint and suffix | Exact admitted checkpoint, then fresh replay | New prefix, unknown |
| Failed/cancelled/ambiguous send | Preserve completed native state | Retire affected cursor; next explicit continuation replays | No blind send/action retry |
| Native tool still running or awaiting approval | Settle or interrupt through Core | Route change waits for native boundary | No re-execution of old call |
| Unsupported model/effort/context format | Preserve | Refuse before inference | No silent substitution |

Let P be stable prefix bytes, H the remaining canonical history, D newly added
bytes, K the committed summary and W protocol-envelope overhead. Logical send
sizes are warm append D+W, cold replay P+H+D+W, and checkpoint replay P+K+D+W.
Configuration replacements add their actual byte size. If P=8KB, H=16KB, D=0.4KB
and K=2KB, these are 0.4KB+W, 24.4KB+W and 10.4KB+W respectively. These are
illustrative serialization costs, not server token/billing measurements. Actual
staging adds one envelope per packet. Prompt cache hits can reduce server work
without reducing wire bytes; loss of a hit must not erase native history.
Published API rates and cache-hit diagnostics cannot price this signed-app Chat
path. Keep observed usage, local estimates and unknown cache state separate.

The implementation now keys the existing prefix cursor by route and connection
generation, commits it only after actual correlated delivery, and replays on
boundary changes. Returning A→B→A replays unless A's context is genuinely still
resident; no unbounded per-model tabs, cache daemon or parallel session database
is added. GPT-6 Instant/Medium/High/Extra High have direct-profile tool-loop
qualification; Pro inference remains prohibited and GPT-5.6 deferred. Actual
shared-backend `/model` inference and long-thread provider transitions remain
unqualified despite the earlier menu-only gate. A family selector and an
effort selector are required by the user; naming all of them Instant is not a
substitute. UI modes are not assumed to equal API reasoning enum values.

Context staging keeps complete ordered schemas and messages in bounded packets,
combining instructions, tool-list replacement and history when they fit. Only the
last packet requests actual work; previous packets remain inert setup. A tool-list
reset shares the first batch rather than consuming a separate empty-list ACK.
Large instruction/message fragments retain their exact order and UTF-16 boundaries;
an oversized whole tool schema refuses before submission. The serialized packet
budget includes its phase marker. Packing never drops context or repeats native
effects. Packet counts describe potential Chat submissions, not quota deductions.
The proof keeps legacy `submittedRequests` as work submissions. New
`submittedContextRequests` records setup before ACK and `observedChatSubmissions`
includes both. `submissionSources` separates app acceptance from weaker GUI
observation; `contextPackets` counts completed ACKs and `retryAttempts` counts retry
attempts. Neither measures extra messages or quota deductions. Authenticated
`arrivals`/`admissionFailures` precede admitted work, with fixed codes and no private
parser details. These changes are source-only until real qualification.
The existing --profile workflow remains the fallback. The on-demand lifecycle
below now supplies durable app/adapter bootstrap without refreshing config paths.
Normal /model family/effort integration remains a separate acceptance task.

Sources: [cache diagnostics](https://developers.openai.com/api/docs/guides/prompt-caching/diagnostics),
[ChatGPT model availability](https://learn.chatgpt.com/docs/models?surface=web).

`desktop_draft_probe.mjs` tests the actual owned GUI while no generation or
user draft is present. It inserts one generated fixture, closes its inspection
connection before invoking the real transport (the relay permits one CDP peer),
requires refusal before send with the fixture unchanged, then clears only the
exact normalized fixture and records booleans. It never uses a user's draft as
test input. Connection/route transitions, immutable boundary snapshots and stale
owner refusal pass18 protocol tests/85 assertions; unsupported route HTTP probes
return400 with no inference. Real catalog/tool execution, automatic compaction,
four native shell calls and persisted restart/resume also pass this foundation.

On-demand lifecycle contract (experimental, separate from stable Core)

The opt-in configuration profile may use the upstream command-backed auth hook
for bootstrap. Its private loopback address/capability stay fixed across adapter
restart; the private runtime record is the one installation authority. Source,
Bun, exported pinned PRoot and noVNC assets live under XDG lib; only disposable
GUI sockets/control files live in TMPDIR. The existing signed package cache and
persistent signed-app login home remain separate and unchanged. The bridge
parser/tool mapper/SSE exports are bundled from the exact qualified bridge source.

Bootstrap is bounded and serialized, starts only the selected trial processes,
and checks PID/start/executable ownership before using any local listener. An
unknown occupied port is refused without sending it the capability or killing
its owner. Existing live controllers are preserved. The adapter resolves the
current private controller marker each request; a changed app generation retires
its cursor and replays the canonical Core projection. Recovery starts before
submission and never retries an ambiguous inference or native action. Expired
login, unsupported model/mode, user drafts and foreign ownership remain errors.
No boot service, alternate Core launcher or second session database is added.
Ordinary Codex without this explicit profile does not bootstrap the trial.

`install_desktop_provider.py` installs from local pinned dependencies and migrates
only this experiment's marked config profiles. `desktop_lifecycle.py token` is
an auth command: its stdout is private credential material, never a diagnostic.
Use `ensure` for a credential-free health result. Settling and stopping the old
owned adapter/controller is a separate bounded migration step; the installer
never kills a running job. Updating this opt-in installation preserves its
address, login home, Core transcript and unchanged base config.


Device lifecycle acceptance now covers the installed --profile command after
adapter termination and whole owned controller termination: each turn completed
actual write/read shell calls and exact final reply. Six simultaneous bootstrap
clients all succeeded through one adapter. Base config, marked trial config and
existing auth hashes stayed unchanged throughout the fault test. Permanent
payload-only reinstall (no TMPDIR dependency sources supplied) reused the exact
same installation twice. Aggregate proof is native-provider/lifecycle-proof.json;
source-installed migration proof is lifecycle-migration-proof.json. The concurrency
regression was a false foreign-owner diagnosis of TIME_WAIT, fixed by matching
SO_REUSEADDR. Foreign listening sockets still refuse before startup; the readiness
challenge test proves no bearer is sent to a fake listener. Local tests include
12 lifecycle/profile and5 desktop/browser routing cases; repository Python18+59
and protocol18/85 pass. Hours-long soak, physical phone reboot and expired-login
recovery remain distinct from these bounded process-fault tests.

For reinstall from the repository or installed source, run the maintained installer
with `--home CODEX_HOME` (repeat for each owned marked home). Without dependency
arguments it verifies/reuses the current installed payload and permanent catalog.
First migration supplies all four qualified bridge/PRoot/noVNC/Bun paths and a
catalog template. No user auth is copied into the payload. Dependencies and the
accepted effective bridge source/licenses remain available without temporary
checkouts. Authentication helper stdout must never be used as a health diagnostic.


Native checkpoint recovery also passes the physical app-generation boundary:
`siwc_core_probe.py --compact --restart-desktop` learns a secret only from native
tool output, removes its original owned fixtures, commits the actual summary,
executes two more native tools, restarts/resumes Core, stops the verified owned
controller and recalls the secret after automatic app recovery without another
tool call. The adapter process itself stays alive through that GUI restart.
Aggregate proof: native-provider/lifecycle-checkpoint-proof.json. Four actual
native commands and the admitted persisted checkpoint pass; estimated token
counts remain local sizing only, and no server KV retention is assumed.


Experimental model selection (not yet production native frontend admission)

The installed profile now uses the observed Chat family GPT-6, with a separate
GPT-5.6 Sol option. Actual UI selection/readback passes Instant, Medium, High and
Extra High in both families. The names map to native none/medium/high/xhigh; the
UI's internal `standard`, `extended` and `max` IDs are not API effort names.
GPT-6 Sol belongs to Work/Codex, so it is not fabricated as a Chat choice. The
legacy Instant wire slug remains a hidden explicit GPT-6/Instant alias.

Each family has a separate Pro entry described as execution unverified. The user
explicitly prohibits real Pro qualification because each request uses its quota.
All maintained native inference drivers assert non-Pro slugs and override their
model explicitly, even if a user later persists Pro as a profile default. Pro was
neither selected nor submitted in physical qualification. Static route validation
and observed UI metadata are its only proof; they do not certify inference.

Actual installed GPT-6/Instant and Medium native qualification each passes two
write/read shell calls and exact correlated final reply
(gpt6-{instant,medium}-proof.json). GPT-6 is the default and current qualification
priority; GPT-5.6 Sol is explicitly deferred in the picker description. It reached
one real shell call but its following response timed out in Instant and Medium;
those failed gates remain gpt56-{instant,medium}-proof.json and are not acceptance.
Creating temporary Chat replaces the UI asynchronously: selection now waits for
the real selector, instead of trying to click it before the view is ready.
Reasoning-mode reply waits allow a bounded ten minutes; ambiguous failures still
retire the cursor without replaying a native action automatically.

The GPT-6 checkpoint fault rerun passes four actual native calls, committed
tool-only memory, Core restart/resume and whole signed-app restart
(gpt6-checkpoint-proof.json). The initial app restart failed on a restored open
selector. The corrected path first verifies a fresh empty Temporary Chat and
reuses an already expanded verified model menu instead of toggling it closed.

`model-menu.patch` is a separate frontend overlay on the accepted native.patch.
It appends the installed marked Chat catalog to the upstream /model picker and
retains the ordinary model/effort UI. Chat choices use native durable
thread persistence, cleanup, then Core re-entry in the same home/window; pending
input/tools/side conversations refuse. The selected configuration profile and
its settings stay selected, including named permission definitions and MCP
configuration. Only command-backed nonsecret Chat routing references enter
overrides. Chat choices are session-only and never persist a Chat slug into an
ordinary Codex default. Named or legacy permissions and the reviewer are carried
explicitly; returning from standalone --profile chatgpt uses the original/base
configuration. It does not pretend that upstream thread/settings/update can change
provider. Native build, unit/lint and actual device transition remain required.
`web-model-menu-native.yml` qualifies the exact official0.161 source plus both
frontend patches without publishing or promoting a production runtime.

The experimental Core keeps the admitted frontend on the official app-server
for local interactive profile/config overrides. Selected profile and invocation
settings go through the upstream per-thread configuration API, including provider
references and named permission definitions; daemon defaults and other threads
are unchanged. No source-built embedded backend or extra server is introduced.
Explicit --no-daemon/remote/executor routes, unsupported workspace flags and
noninteractive commands retain the ordinary official runtime path. Optional
Manager/frontend absence still leaves Core usable. Device round-trip proof and
full acceptance remain required; installed signed stable49 is unchanged.

A settled temporary Chat can return to the desktop Home page. The adapter uses
the public Ctrl+Shift+N shortcut only on a verified Home surface with no visible
composer or Stop control; ordinary drafts stay protected. Quick Chat is not a
model transport. Full-access harmless shell commands may require no approval,
so the autoreviewer fixture installs a prompt rule only in a disposable config
home and requires a real guardian request as well as the two native calls.

Native threads and guardian children have independent temporary Chat
conversations, ledgers and prompt cursors. The signed app's exported submit,
cancel and per-conversation state replace composer typing/Send clicking.
Its normal Chat origin is omitted (the literal `chat` is not a valid wire origin).
Each continuation explicitly parents its next submission to the owned completed
correlated assistant reply. Internal startup waits for the app Scope rather than
misclassifying a restoring app as an absent capability. Both transports share the
qualified external-tool instructions.

App authentication and token refresh stay in the app. A shared private CDP
connection carries concurrent operations without introducing a debug TCP port.
The app and display are still dependencies; hidden execution is not a claim
that Electron was removed. Cancellation aborts only the exact owned conversation.
Unsupported internal discovery can fall back before submission to the prior UI
path, with a single owner and guardian replay; ambiguous submission failures
never switch transports or repeat an external native action.

`desktop_app_probe.mjs` requires two exact overlapping GPT-6 replies and a
cancelled lane whose sibling survives. `desktop_native_concurrency_probe.py`
requires two actual Android Core clients, four native commands, isolated exact
markers/replies, global profile projection and unchanged base config. Its
`--autoreview` variant also requires actual native guardian requests in an
independent Chat conversation. Both prohibit Pro inference. A first guardian
trial had one incomplete sibling; it remains failed evidence. The bounded retry
passed both clients and two actual guardian requests. Reports live in the private
acceptance cache, not this source tree.

The GPT-6 owned-rule gate now passes one actual native guardian review, one
parent replay, two approved native shell operations and exact final reply with
zero owner conflicts. gpt6-autoreview-rule-proof.json records the aggregate.
The same adapter also passes four native calls, actual summary persistence and
Core plus whole signed-app restart in gpt6-checkpoint-guardian-lanes-proof.json.
The strengthened native menu fixture reads the official backend's thread/model/
provider, rather than treating argv or menu rendering as proof of routing.
Its real-file regression exposed Value::from_str parsing a scalar instead of a
TOML document in the pinned0.9 dependency; installed routing uses toml::from_str.
Native return explicitly chooses OpenAI because an omitted provider can restore
the persisted Chat route during resume. Device round-trip acceptance remains
required before production admission.

Historical native qualification38510380995 atd1b433c passes build,35 actual TUI tests,
lint and pristine upstream Core checks. On the phone, native_model_menu_probe.py
passes both base configuration and --profile owned-config/named permissions:
/model enters Chat GPT-6/Instant and returns to native OpenAI, with the same
PID/PTY/home/thread and unchanged configuration files. The official backend's
loaded thread confirms both provider/model changes, and an independent ephemeral
thread retains server defaults. No model inference or production activation is
performed by this menu fixture. Pro is never selected by its model-name matcher.
The fixture completes upstream screen-reader initialization before hashing config,
so an ordinary first-launch UI preference write is not confused with model persistence.
Private aggregate proofs: ~/.cache/codex/acceptance/chat-model-menu-device-legacy.json
and chat-model-menu-device-named.json. Its qualification-only Unix WebSocket
reader uses websockets17.2 in isolated XDG cache, not the provider runtime:

```sh
python -m pip install --no-deps --no-compile \
  --target "${XDG_CACHE_HOME:-$HOME/.cache}/codex/acceptance/ws-client-17.2" websockets==17.2
```

GPT-6 High and Extra High also pass two actual native shell operations and exact
final reply using the installed --profile chatgpt route. Instant, Medium, High
and Extra High now all have actual native tool-loop evidence; Pro remains
execution-unverified and GPT-5.6 remains deferred. Private reports:
gpt6-high-proof.json and gpt6-xhigh-proof.json in the same acceptance cache.
Formal production admission of the model overlay remains separate; signed
stable49 and existing user jobs retain their current binaries.


Current combined native qualification38552383479/source5f8ff12 passes39 actual TUI
regressions and is published/installed as signed stable51. Provider source46db05e
also passes real manual compaction with four native commands and Core/whole-app
restart. Actual GitHub discovered-tool execution passes in the wrlab native home;
szob has no GitHub catalog matches. Shared Chat app login does not confer native
connector permissions. Concurrent native clients and guardian children keep their
independent conversations; app acceptance timestamps, rather than polling time,
measure overlap. See CURRENT.md for the current qualified scope and private proofs.

Current source-only admission repairs accept identity/zstd bodies up to16 MiB
encoded and32 MiB expanded, validate UTF-8 and distinguish malformed JSON,
compression and unsupported encoding before Chat submission. Stored opaque or
corrupt compaction envelopes are rejected before the shared parser can replace
them with a placeholder. This prevents silent context loss; it does not make
native encrypted checkpoints readable. Installed provider46db05e retains the old
2 MiB ceiling until separate activation and affected live qualification.

The offline `desktop_broker_probe.mjs` exercises the existing pinned broker's Unix
IPC without a tunnel or model; it proves result resolution, replay and observer/
revoke isolation, not live Chat continuation. `native_catalog_request_probe.py`
uses the installed official backend against local fake endpoints only. It now
also proves merged startup catalog admission and actual turn/interrupt stream
closure. The provider receives no separate interruption request, so stream closure
cannot be ignored until a qualified explicit cancellation path exists. Neither
fixture starts real Chat inference or changes an account's settings/history.
