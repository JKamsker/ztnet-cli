#!/usr/bin/env python3
from __future__ import annotations

import os
import subprocess


def main() -> int:
    token = os.environ.get("CARGO_REGISTRY_TOKEN", "")
    if not token.strip():
        print("CARGO_REGISTRY_TOKEN is not set; skipping cargo publish")
        return 0

    subprocess.run(["cargo", "publish", "--locked"], check=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

