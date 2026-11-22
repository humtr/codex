#!/usr/bin/env python3
"""Build the optional Termux tunnel-client from official source, without another checkout.

Release preparation and explicit source-build recovery use this same recipe.
Normal installation consumes signed prebuilt packages; this command never
selects/restarts a service, edits credentials or publishes an artifact.
Supports GitHub API 403 rate-limit fallback via web redirect and optional
Linux->Android NDK cross-compilation (`--allow-cross`).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile
import urllib.error
import urllib.request

RECIPE_ROOT = Path(__file__).resolve().parents[1] / "transport/tunnel-client"
LATEST_API = "https://api.github.com/repos/openai/tunnel-client/releases/latest"
LATEST_WEB = "https://github.com/openai/tunnel-client/releases/latest"
TAG_RE = re.compile(r"^v[0-9]+\.[0-9]+\.[0-9]+$")


def run(argv, *, cwd=None, env=None, capture=False):
    return subprocess.run(
        list(map(str, argv)),
        cwd=cwd,
        env=env,
        check=True,
        stdout=subprocess.PIPE if capture else None,
        text=capture,
    ).stdout


def sha(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def parse_redirect_tag(location: str) -> str:
    marker = "/releases/tag/"
    if marker not in location:
        raise ValueError(f"unexpected GitHub releases redirect location: {location!r}")
    tag = location.split(marker, 1)[1].split("?", 1)[0].split("#", 1)[0].strip("/")
    if not TAG_RE.fullmatch(tag):
        raise ValueError(f"unsupported release tag from redirect: {tag!r}")
    return tag


def fetch_latest_web_redirect(web_url: str = LATEST_WEB) -> dict:
    opener = urllib.request.build_opener(_NoRedirect)
    req = urllib.request.Request(web_url, method="HEAD", headers={"User-Agent": "codex-termux"})
    try:
        with opener.open(req, timeout=20) as resp:
            loc = resp.headers.get("Location", "")
    except urllib.error.HTTPError as exc:
        if exc.code in (301, 302, 303, 307, 308):
            loc = exc.headers.get("Location", "")
        else:
            raise ValueError(f"GitHub web fallback failed with HTTP {exc.code}") from exc
    tag = parse_redirect_tag(loc)
    return {"tag_name": tag, "draft": False, "prerelease": False, "via_redirect": True}


def fetch_latest_release(
    api_url: str = LATEST_API,
    web_url: str = LATEST_WEB,
    api_opener=None,
    web_fallback=None,
) -> dict:
    fallback = web_fallback or (lambda: fetch_latest_web_redirect(web_url))
    try:
        if api_opener is not None:
            return api_opener(api_url)
        req = urllib.request.Request(
            api_url,
            headers={"Accept": "application/vnd.github+json", "User-Agent": "codex-termux"},
        )
        with urllib.request.urlopen(req, timeout=20) as resp:
            raw = resp.read(1048576)
        return json.loads(raw.decode("utf-8"))
    except urllib.error.HTTPError as exc:
        code = exc.code
        exc.close()
        if code == 403:
            return fallback()
        raise


def select_release(requested: str, recipe: dict, fetch) -> str:
    version = requested
    if requested == "latest":
        release = fetch()
        if release.get("draft") or release.get("prerelease"):
            raise ValueError("official stable release metadata is not stable")
        version = release.get("tag_name")
    if version != recipe["version"]:
        raise ValueError(
            f"upstream {version!r} is outside the validated recipe "
            f"{recipe['version']}; update the recipe; no older build was selected"
        )
    return version


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--version",
        default="latest",
        help="latest official stable or exact validated version",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="absent absolute artifact directory; no service activation",
    )
    parser.add_argument(
        "--allow-cross",
        action="store_true",
        help="permit Linux host -> Android/arm64 NDK cross-compile (CI prebuilt lane)",
    )
    parser.add_argument(
        "--cc",
        default=None,
        help="explicit C compiler path (e.g. aarch64-linux-android24-clang for --allow-cross)",
    )
    args = parser.parse_args()
    if not args.output.is_absolute() or args.output.exists():
        parser.error("--output must name an absent absolute directory")
    recipe = json.loads((RECIPE_ROOT / "recipe.json").read_text())
    patch = RECIPE_ROOT / "termux.patch"

    for tool in ("git", "go"):
        if shutil.which(tool) is None:
            parser.error(f"missing required build tool: {tool}")
    cc_path = args.cc or shutil.which("clang")
    if not cc_path or not Path(cc_path).is_file():
        parser.error("missing C compiler (clang or --cc <android-ndk-clang>)")

    go_target = json.loads(run(["go", "env", "-json", "GOOS", "GOARCH"], capture=True))
    native_android = go_target == {"GOOS": "android", "GOARCH": "arm64"}
    if not native_android:
        if not args.allow_cross:
            parser.error(
                "an actual Android/arm64 Go toolchain is required (or pass --allow-cross --cc <ndk-clang>)"
            )
        if platform.system() != "Linux":
            parser.error("--allow-cross is supported only on Linux hosts")

    version = select_release(args.version, recipe, fetch_latest_release)

    tmp_base = os.environ.get("TMPDIR") or tempfile.gettempdir()
    work = Path(tempfile.mkdtemp(prefix="codex-tunnel-build-", dir=tmp_base))
    print("Tunnel build workspace: " + str(work), flush=True)
    source = work / "source"
    source.mkdir()
    cache = Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "codex"
    home = work / "home"
    home.mkdir(mode=0o700)
    module_cache = run(["go", "env", "GOMODCACHE"], capture=True).strip()

    base_env = {
        key: os.environ[key]
        for key in ("PATH", "PREFIX", "TMPDIR", "LANG", "SSL_CERT_FILE")
        if key in os.environ
    }
    base_env.update(
        HOME=str(home),
        GOCACHE=str(cache / "tunnel-go-build"),
        GOMODCACHE=module_cache,
        GOMAXPROCS="2",
        GOFLAGS="-p=1",
        GOTOOLCHAIN="local",
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL="/dev/null",
        GIT_TERMINAL_PROMPT="0",
    )

    run(["git", "init", "-q", source], env=base_env)
    run(
        [
            "git",
            "-c",
            "protocol.file.allow=never",
            "fetch",
            "--no-tags",
            "--depth=1",
            recipe["upstream_repository"],
            recipe["commit"],
        ],
        cwd=source,
        env=base_env,
    )
    actual = run(["git", "rev-parse", "FETCH_HEAD"], cwd=source, env=base_env, capture=True).strip()
    if actual != recipe["commit"]:
        raise ValueError("official source commit differs from the validated recipe")
    run(["git", "checkout", "-q", "--detach", actual], cwd=source, env=base_env)
    if (source / "pkg/version/VERSION").read_text().strip() != version[1:]:
        raise ValueError("official source version differs from selected release")
    run(["git", "apply", "--check", "--whitespace=error", patch], cwd=source, env=base_env)
    run(["git", "apply", "--whitespace=error", patch], cwd=source, env=base_env)

    # Run Go tests on the executing host (native Android/arm64 on Termux, or Linux host in CI).
    test_env = dict(base_env)
    if native_android:
        test_env.update(recipe["target"], CC=cc_path)
    run(
        [
            "go",
            "test",
            "./pkg/controlplane/internal",
            "-count=1",
            "-run",
            recipe["regression_test"],
        ],
        cwd=source,
        env=test_env,
    )
    run(["go", "test", *recipe["test_packages"]], cwd=source, env=test_env)

    # Build final android/arm64 binary with immutable version + patch identity.
    build_env = dict(base_env)
    build_env.update(recipe["target"], CC=cc_path)
    if args.allow_cross:
        build_env["CGO_LDFLAGS"] = "-Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384"
    binary = work / "tunnel-client"
    identity = "github.com/openai/tunnel-client/pkg/version."
    recipe_sha, patch_sha = sha(RECIPE_ROOT / "recipe.json"), sha(patch)
    derived_version = version[1:] + "-termux." + patch_sha[:12]
    flags = f"-s -w -X {identity}GitSHA={actual} -X {identity}semanticVersion={derived_version}"
    run(
        [
            "go",
            "build",
            "-trimpath",
            "-buildvcs=false",
            "-ldflags=" + flags,
            "-o",
            binary,
            "./cmd/client",
        ],
        cwd=source,
        env=build_env,
    )

    observed = None
    if native_android:
        observed = run([binary, "--version"], capture=True).strip()
        if not observed.startswith(derived_version + "+") or actual not in observed:
            raise ValueError("built executable did not report upstream and patch identity")

    metadata = run(["go", "version", "-m", binary], capture=True)
    for key, value in recipe["target"].items():
        if "\t" + key + "=" + value not in metadata:
            raise ValueError("built executable target mismatch: " + key)

    output = args.output
    output.mkdir(mode=0o700)
    try:
        shutil.copy2(binary, output / "tunnel-client")
        (output / "tunnel-client").chmod(0o755)
        shutil.copy2(source / "LICENSE", output / "LICENSE")
        declaration = dict(
            upstream_version=version,
            upstream_commit=actual,
            recipe_sha256=recipe_sha,
            patch_sha256=patch_sha,
            binary_sha256=sha(binary),
            target=recipe["target"],
            cross_compiled=not native_android,
            go=run(["go", "version"], capture=True).strip(),
            version_output=observed,
            tests=recipe["test_packages"],
            activation_performed=False,
        )
        (output / "build.json").write_text(json.dumps(declaration, indent=2) + "\n")
    except BaseException:
        print("Incomplete build output retained at " + str(output), flush=True)
        raise
    shutil.rmtree(work)
    print(json.dumps(dict(accepted=True, output=str(output), **declaration)), flush=True)


if __name__ == "__main__":
    os.umask(0o077)
    main()
