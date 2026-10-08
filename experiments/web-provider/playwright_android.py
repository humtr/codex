"""Bounded Android cache correction for the exact owned Playwright dependency."""
import argparse
import hashlib
import json
import os
import tempfile
from pathlib import Path

BEFORE = '3258d1cf334c6afc95f22aa9c292436cb976b391e0437f1359c83b84f0cb9d66'
AFTER = '4952f2e7ddbfe0e8039da98a236d9a2ae66f0cee0c00cee507eb9605a3bf6c3f'
OLD = (b'    defaultCacheDirectory = (() => {\n      if (process.platform === "linux")', b'    defaultCacheDirectory2 = (() => {\n      if (process.platform === "linux")', b'      let localCacheDir;\n      if (process.platform === "linux")')
NEW = (b'    defaultCacheDirectory = (() => {\n      if (process.platform === "linux" || process.platform === "android")', b'    defaultCacheDirectory2 = (() => {\n      if (process.platform === "linux" || process.platform === "android")', b'      let localCacheDir;\n      if (process.platform === "linux" || process.platform === "android")')


def corrected_bundle(data):
    digest = hashlib.sha256(data).hexdigest()
    if digest == AFTER:
        return data
    if digest != BEFORE or any(data.count(old) != 1 for old in OLD):
        raise ValueError("Pinned Playwright dependency identity mismatch")
    result = data
    for old, new in zip(OLD, NEW):
        result = result.replace(old, new)
    if hashlib.sha256(result).hexdigest() != AFTER:
        raise ValueError("Corrected Playwright identity mismatch")
    return result


def apply(candidate):
    candidate = candidate.resolve(strict=True)
    temp = Path(tempfile.gettempdir()).resolve()
    if not candidate.is_relative_to(temp):
        raise ValueError("Candidate must be inside the owned temporary root")
    package = candidate / "node_modules/playwright-core"
    if json.loads((package / "package.json").read_text())["version"] != "1.62.0":
        raise ValueError("Pinned Playwright version mismatch")
    target = package / "lib/coreBundle.js"
    if not target.resolve().is_relative_to(candidate):
        raise ValueError("Dependency target escaped candidate")
    original = target.read_bytes()
    corrected = corrected_bundle(original)
    if corrected != original:
        fd, name = tempfile.mkstemp(prefix=".android-cache-", dir=target.parent)
        try:
            with os.fdopen(fd, "wb") as output:
                output.write(corrected)
            os.chmod(name, target.stat().st_mode & 0o777)
            os.replace(name, target)
        finally:
            Path(name).unlink(missing_ok=True)
    return {"playwright": "1.62.0", "before": BEFORE, "after": AFTER,
            "scope": "three-startup-caches-only", "platform_spoof": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", type=Path, required=True)
    print(json.dumps(apply(parser.parse_args().candidate), sort_keys=True))
