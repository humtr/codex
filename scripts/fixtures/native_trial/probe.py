"""Owned, repeat-sensitive native tool probe. Does not contact a model."""
import argparse
import os
import time
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument("--root", type=Path, required=True)
p.add_argument("--label", choices=["fault", "long"], required=True)
p.add_argument("--delay", type=float, default=0)
a = p.parse_args()
a.root.mkdir(parents=True, exist_ok=True)
fd = os.open(a.root / (a.label + ".effects"), os.O_CREAT | os.O_APPEND | os.O_WRONLY, 0o600)
with os.fdopen(fd, "a") as stream:
    stream.write(a.label + "\n")
    stream.flush()
    os.fsync(stream.fileno())
time.sleep(a.delay)
print("D3_EFFECT_RETURNED_" + a.label.upper(), flush=True)
