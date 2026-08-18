#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "pillow",
# ]
# ///
"""Generate the demo icons and run the OoT kaleidoscope pause-menu demo."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def main() -> int:
    repo_root = Path(__file__).resolve().parent
    env = {**os.environ, "RUST_BACKTRACE": os.environ.get("RUST_BACKTRACE", "full")}

    subprocess.run(
        [sys.executable, "tools/generate_oot_demo_icons.py"],
        cwd=repo_root,
        env=env,
        check=True,
    )
    return subprocess.call(["cargo", "run", "--release"], cwd=repo_root, env=env)


if __name__ == "__main__":
    raise SystemExit(main())
