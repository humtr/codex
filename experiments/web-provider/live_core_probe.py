"""Bounded real Web provider through an admitted Core; synthetic task, real service.

The existing human browser is untouched. Separate X11/HTTP/Core state is owned
and removed on exit; authentication is read only from the experimental login.
No request/reply body, credentials, screenshots or challenge solving is recorded.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import re
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import time

SOURCE = Path(__file__).resolve().parent
sys.path.insert(0, str(SOURCE.parents[1] / "scripts"))
from qualify_tasks import fixture_activation_state, materialize_signed_generation
from rald3_preflight import parse_descriptor
from rald5_publication import parse_manifest


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def probe(args):
    host_prefix = Path(os.environ["PREFIX"])
    real_home = Path.home()
    activation = dict(line.split("=", 1) for line in (real_home / ".local/share/codex/core/activation-state").read_text().splitlines() if "=" in line)
    generation = real_home / ".local/lib/codex/core/generations" / activation["current"]
    fields = parse_descriptor(generation / "generation.meta")
    _, inventory = parse_manifest(generation / "release.manifest")
    before_auth = {str(p): sha(p) for p in [args.storage, Path(str(args.storage) + ".verified.json")]}
    assert json.loads(Path(str(args.storage) + ".verified.json").read_text()).get("authenticated") is True
    assert not args.broker.exists(), "another trial owns the execution broker"
    proof = {"realService": True, "nativeCoreGeneration": fields["generation_id"], "coreExecutionPath": True, "requestedWebModelSlug": "chatgpt-web/luna",
             "realModelReplyAccepted": False, "realModelToolLoopAccepted": False, "captchaSolvingAttempted": False}
    processes = []
    external_owner = None
    model_slug = "chatgpt-web/luna"
    capabilities = {"solAvailable": False, "extraHighAvailable": False, "proAvailable": False}
    with tempfile.TemporaryDirectory(prefix="web-live-core-", dir=args.parent) as temporary:
        root = Path(temporary)
        root.chmod(0o700)
        home, prefix, workspace = root / "h", root / "p", root / "w"
        for p in [home / ".codex", prefix / "bin", prefix / "etc/tls", workspace, home / ".local/share/codex/core/config", root / "t"]:
            p.mkdir(parents=True, exist_ok=True, mode=0o700)
        # Trust the active Core's admitted key; verify the signed inventory before copying.
        der = root / "key.der"
        der.write_bytes(bytes.fromhex("302a300506032b6570032100" + activation["current_key"]))
        public = root / "public.pem"
        subprocess.run(["openssl", "pkey", "-pubin", "-inform", "DER", "-in", str(der), "-out", str(public)], check=True, capture_output=True)
        subprocess.run(["openssl", "pkeyutl", "-verify", "-pubin", "-inkey", str(public), "-rawin", "-in", str(generation / "release.manifest"), "-sigfile", str(generation / "release.sig")], check=True, capture_output=True)
        for name, expected, mode in inventory:
            assert sha(generation / name) == expected and (generation / name).stat().st_mode & 0o777 == int(mode, 8)
        installed = home / ".local/lib/codex/core/generations" / fields["generation_id"]
        materialize_signed_generation(generation, installed)
        launcher = prefix / "bin/codex"
        shutil.copy2(generation / "core", launcher)
        (prefix / "bin/openssl").symlink_to(shutil.which("openssl"))
        shutil.copy2(host_prefix / "etc/tls/cert.pem", prefix / "etc/tls/cert.pem")
        shutil.copy2(host_prefix / "etc/resolv.conf", prefix / "etc/resolv.conf")
        (home / ".local/share/codex/core/activation-state").write_text(fixture_activation_state(fields["generation_id"], public))
        display = next(n for n in range(128, 256) if not (args.parent / f".X{n}-lock").exists() and not (args.parent / ".X11-unix" / f"X{n}").exists())
        def field(value): return struct.pack("!H", len(value)) + value
        authority = root / "xauthority"
        authority.write_bytes(struct.pack("!H", 65535) + field(b"localhost") + field(str(display).encode()) + field(b"MIT-MAGIC-COOKIE-1") + field(secrets.token_bytes(16)))
        authority.chmod(0o600)
        base_env = {"HOME": str(home), "PREFIX": str(host_prefix), "PATH": str(host_prefix / "bin"), "TMPDIR": str(root / "t"),
                    "LANG": "C.UTF-8", "DISPLAY": f":{display}", "XAUTHORITY": str(authority), "CODEX_HOME": str(home / ".codex"),
                    "CODEX_CHATGPT_WEB_HOME": str(root / "web"), "XDG_CACHE_HOME": str(home / ".cache"), "XDG_CONFIG_HOME": str(home / ".config")}
        def start(argv, log):
            with (root / log).open("w") as out:
                p = subprocess.Popen(argv, env=base_env, stdout=out, stderr=out, start_new_session=True)
            processes.append(p)
            return p
        xvnc = None if args.existing_native_descriptor else start([str(host_prefix / "bin/Xvnc"), f":{display}", "-geometry", "1280x800", "-depth", "24", "-rfbunixpath", str(root / "rfb.sock"), "-rfbunixmode", "0600", "-rfbport", "-1", "-SecurityTypes", "None", "-auth", str(authority), "-nolisten", "tcp"], "xvnc.log")
        try:
            for _ in range(50):
                if xvnc is None or (root / "rfb.sock").exists(): break
                assert xvnc.poll() is None, "owned X11 startup failed"
                time.sleep(.1)
            else: raise RuntimeError("owned X11 readiness timed out")
            if xvnc is not None: start([str(host_prefix / "bin/openbox")], "openbox.log")
            native_descriptor = None
            if args.native_host:
                if args.existing_native_descriptor:
                    native_descriptor = args.existing_native_descriptor.resolve(strict=True)
                    external_owner = json.loads(native_descriptor.read_text())
                    proof["externalNativeBrowserUsed"] = True
                else:
                    profile = args.storage.parent / "native-pipe-profile"
                    profile.mkdir(mode=0o700, exist_ok=True)
                    with tempfile.TemporaryDirectory(prefix="cs-", dir=args.parent) as short:
                        seed_env = dict(base_env, TMPDIR=short)
                        result = subprocess.run([str(args.bun), str(SOURCE / "seed_manual_profile.mjs"), str(args.candidate), str(profile), str(args.storage)], env=seed_env, capture_output=True, timeout=40)
                        assert result.returncode == 0, "experimental native profile seeding failed"
                    native_descriptor = root / "native-host.json"
                    owner = start([str(args.bun), str(SOURCE / "native_chromium_host.mjs"), str(profile), str(native_descriptor)], "native-host.log")
                    for _ in range(150):
                        if native_descriptor.exists(): break
                        assert owner.poll() is None, "native persistent browser host failed"
                        time.sleep(.1)
                    else: raise RuntimeError("native persistent browser host readiness failed")
                if args.prepared_native_session:
                    assert args.existing_native_descriptor and external_owner.get("warmPageTargetId"), "prepared page must belong to the external native host"
                    session = json.loads(args.prepared_native_session.read_text())
                    assert session.get("emptyPreparedPageRetained") and session.get("authenticated") and session.get("composerCount") == 1
                    proof["preparedPageInspectionReused"] = True
                else:
                    ready = subprocess.run([str(args.bun), str(SOURCE / "native_session_probe.mjs"), str(args.candidate), str(native_descriptor), str(root / "native-session.json"), "--retain-page"], env=base_env, capture_output=True, timeout=45)
                    assert ready.returncode == 0 and (root / "native-session.json").exists(), "native session inspection failed"
                    session = json.loads((root / "native-session.json").read_text())
                proof["nativeSession"] = session
                if not session["composerReady"]:
                    proof["blockedBeforeCoreRequest"] = True
                    return proof
                capabilities = session["capabilities"]
                model_slug = session["currentUiModelSlug"]
                proof["requestedWebModelSlug"] = model_slug
            # Actual candidate server and adapter; no fake browser/Responses implementation.
            gateway = root / "gateway.mjs"
            gateway.write_text(f'''import {{defaultConfig}} from {json.dumps(str(args.candidate / "src/config.ts"))};
import {{startServer}} from {json.dumps(str(args.candidate / "src/server.ts"))};
import {{writeFileSync}} from 'node:fs';
const config={{...defaultConfig('full'),port:0,appName:'Codex Native2',automaticAppName:'Codex Native2',chromeExecutablePath:{json.dumps(str(host_prefix / "lib/chromium/chrome"))},storageStatePath:{json.dumps(str(args.storage))},brokerSocketPath:{json.dumps(str(args.broker))},headed:true,...{json.dumps(capabilities)},...{json.dumps({"browserHost": "native-chromium", "browserHostDescriptorPath": str(native_descriptor)} if native_descriptor else {})} ,stallTimeoutSec:60,experimentalFreshConversationPerTurn:true}};
const server=startServer(config);
const observations=[];
const recorder=Bun.serve({{hostname:'127.0.0.1',port:0,idleTimeout:0,async fetch(req){{
 const request=await req.clone().json();
 if(request.model!=={json.dumps(model_slug)})return Response.json({{error:{{type:'invalid_request_error',message:'owned trial requires its explicit Web model route'}}}},{{status:400}});
 const observation={{requestKeys:Object.keys(request),model:request.model,inputTypes:(request.input??[]).map(x=>x.type),toolTypes:(request.tools??[]).map(x=>x.type)}};
 observations.push(observation);
 writeFileSync({json.dumps(str(root / "observations.json"))},JSON.stringify(observations),{{mode:0o600}});
 const response=await fetch('http://127.0.0.1:'+server.port+'/v1/responses',{{method:'POST',headers:req.headers,body:await req.text(),signal:req.signal}});
 observation.status=response.status;
 if(!response.ok){{try{{const error=(await response.clone().json()).error;observation.errorType=error?.type;observation.errorCode=error?.code;observation.errorClasses=['tool','namespace','turn_id','environment','auth','model'].filter(x=>String(error?.message??'').includes(x));}}catch{{}}}}
 writeFileSync({json.dumps(str(root / "observations.json"))},JSON.stringify(observations),{{mode:0o600}});
 return response;
}}}});
process.once('SIGTERM',()=>recorder.stop(true));
writeFileSync({json.dumps(str(root / "gateway.json"))},JSON.stringify({{port:recorder.port}}),{{mode:0o600}});
''')
            gateway_proc = start([str(args.bun), str(gateway)], "gateway.log")
            for _ in range(100):
                if (root / "gateway.json").exists() and args.broker.exists(): break
                assert gateway_proc.poll() is None, "owned gateway startup failed"
                time.sleep(.1)
            else: raise RuntimeError("owned gateway readiness timed out")
            proof["actualCandidateGatewayStarted"] = True
            proof["executionBrokerConnected"] = True
            port = json.loads((root / "gateway.json").read_text())["port"]
            config = f'''model = "{model_slug}"
model_provider = "web_trial"
model_reasoning_effort = "low"
approval_policy = "on-request"
approvals_reviewer = "auto_review"
sandbox_mode = "danger-full-access"
web_search = "disabled"
check_for_update_on_startup = false
[analytics]
enabled = false
[feedback]
enabled = false
[features]
memories = false
[model_providers.web_trial]
name = "Owned real ChatGPT Web trial"
base_url = "http://127.0.0.1:{port}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
'''
            (home / ".codex/config.toml").write_text(config)
            env = dict(base_env, PREFIX=str(prefix), SSL_CERT_FILE=str(prefix / "etc/tls/cert.pem"), TERM="xterm-256color", SHELL=str(host_prefix / "bin/bash"))
            nonce = "WEB_CORE_" + secrets.token_hex(12)
            core = subprocess.Popen([str(launcher), "exec", "--ephemeral", "--skip-git-repo-check", "--json", "-C", str(workspace), "Reply with exactly " + nonce + ". No tools and no additional text."], env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            processes.append(core)
            try: stdout, stderr = core.communicate(timeout=110)
            except subprocess.TimeoutExpired:
                os.killpg(core.pid, signal.SIGINT)
                try: stdout, stderr = core.communicate(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(core.pid, signal.SIGTERM);stdout, stderr = core.communicate(timeout=5)
                proof["boundedCoreTurnTimedOut"] = True
            proof["httpObservations"] = json.loads((root / "observations.json").read_text()) if (root / "observations.json").exists() else []
            proof["coreExit"] = core.returncode
            rows = []
            for line in stdout.splitlines():
                try: rows.append(json.loads(line))
                except ValueError: pass
            answers = [x.get("item", {}).get("text", "") for x in rows if x.get("item", {}).get("type") == "agent_message"]
            proof["realModelReplyAccepted"] = core.returncode == 0 and any(a.strip() == nonce for a in answers)
            proof["coreEventTypes"] = sorted(set(x.get("type", "unknown") for x in rows))
            log = (root / "gateway.log").read_text()
            network = Path(str(native_descriptor) + ".network") if native_descriptor else None
            proof["nativeDocumentStatuses"] = json.loads(network.read_text()) if network and network.exists() else []
            diagnostic_errors = []
            diagnostic_states = []
            for path in (root / "web").rglob("*.json"):
                try:
                    data = json.loads(path.read_text())
                    if isinstance(data, dict) and isinstance(data.get("error"), str):
                        error = data["error"]
                        signatures = {
                            "browserSecurityCheck": "blocked this request with a browser security check",
                            "connectorMenuClosed": "connector menu did not open",
                            "connectorRowMissing": "exposed no row named",
                            "connectorCatalogMissing": "connector catalog is missing",
                            "connectorNotSelected": "did not select",
                            "personalizationMissing": "verifiable personalization control",
                            "personalizationUnavailable": "configured ChatGPT connector remained unavailable",
                            "personalizationMenuUnavailable": "personalization menu",
                            "personalizationDeadline": "personalization preflight exceeded",
                            "personalizationChoiceUnavailable": "personalization choice",
                            "attachmentIntegrity": "did not preserve the complete prompt",
                            "attachmentCleanup": "composer state could not be cleared",
                            "emptyComposerCleanup": "cleanup did not produce an empty composer",
                        }
                        diagnostic_errors.append({"checkpoint": data.get("checkpoint"), "errorClasses": [name for name in ("composer", "timed out", "login", "session", "Temporary", "temporary", "closed", "Target", "page.wait", "page.goto", "Locator", "connector", "personalization", "prompt") if name in error], "staticSignatures": [key for key, value in signatures.items() if value in error], "boundedError": re.sub(r"[A-Za-z0-9_-]{32,}", "[id]", re.sub(r"https?://[^ ]+|/[A-Za-z0-9._/-]+", "[path]", re.sub(r'"[^"\n]*"', '[quoted]', error.splitlines()[0])))[:300]})
                    if isinstance(data, dict) and isinstance(data.get("state"), dict):
                        state = data["state"]; composer = state.get("composer", {})
                        diagnostic_states.append({"checkpoint": data.get("checkpoint"), "composerCount": composer.get("visibleCount"), "composerTextChars": composer.get("textChars"), "selectedConnectorCount": composer.get("exactSelectedConnectorCount"), "matchingConnectorRowCount": len(state.get("connectorRows", [])), "menuCount": len(state.get("menus", [])), "overlayCount": len(state.get("overlays", []))})
                except (ValueError, OSError): pass
            proof["diagnosticErrorClasses"] = diagnostic_errors
            proof["safeBrowserStateCounts"] = diagnostic_states
            proof["browserStageEvents"] = re.findall(r"stage=([a-zA-Z0-9_-]+) (started|completed|failed)", log)
            # Do not classify failures from arbitrary successful-stage log text.
            proof["failureClasses"] = sorted({name for row in diagnostic_errors for name in row["staticSignatures"]})
            proof["diagnosticScreenshotCount"] = len(list((root / "web").rglob("*.png")))
            assert proof["diagnosticScreenshotCount"] == 0
            # Copy only already-sanitized structural diagnostics, never raw gateway logs.
            diagnostics = list((root / "web").rglob("*.json"))
            proof["safeDiagnosticCount"] = len(diagnostics)
        finally:
            for p in reversed(processes):
                if p.poll() is None:
                    p.terminate()
                    try: p.wait(timeout=8)
                    except subprocess.TimeoutExpired: os.killpg(p.pid, signal.SIGKILL);p.wait()
            proof["ownedProcessesExited"] = all(p.poll() is not None for p in processes)
            proof["brokerRemoved"] = not args.broker.exists()
            if args.existing_native_descriptor and external_owner:
                before = external_owner["browser"]
                process = Path("/proc") / str(before["pid"])
                proof["externalNativeBrowserPreserved"] = os.readlink(process / "exe") == before["image"] and process.joinpath("stat").read_text().rsplit(") ", 1)[1].split()[19] == str(before["start"])
                assert proof["externalNativeBrowserPreserved"]
            proof["experimentalAuthenticationUnchanged"] = before_auth == {p: sha(Path(p)) for p in before_auth}
            assert proof["experimentalAuthenticationUnchanged"] and proof["ownedProcessesExited"] and proof["brokerRemoved"]
            args.proof.write_text(json.dumps(proof, indent=2) + "\n")
            args.proof.chmod(0o600)
            if proof.get("blockedBeforeCoreRequest"): print(json.dumps(proof))
    args.proof.write_text(json.dumps(proof, indent=2) + "\n")
    args.proof.chmod(0o600)
    print(json.dumps(proof))
    return 0 if proof["realModelReplyAccepted"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("candidate", "bun", "storage", "broker", "proof"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--existing-native-descriptor", type=Path)
    parser.add_argument("--prepared-native-session", type=Path)
    parser.add_argument("--native-host", action="store_true")
    parser.add_argument("--parent", type=Path, default=Path(os.environ["TMPDIR"]))
    args = parser.parse_args()
    if args.existing_native_descriptor and not args.native_host: parser.error("existing descriptor requires native-host")
    result = probe(args)
    raise SystemExit(0 if isinstance(result, dict) and result.get("realModelReplyAccepted") else result if isinstance(result, int) else 1)


if __name__ == "__main__": main()
