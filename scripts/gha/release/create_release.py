#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Create a GitHub release from dist/* assets.")
    parser.add_argument("--version", required=True)
    parser.add_argument("--dist-dir", default="dist")
    args = parser.parse_args()

    tag = f"v{args.version}"
    dist_dir = Path(args.dist_dir)
    assets = sorted(p for p in dist_dir.glob("*") if p.is_file())
    if not assets:
        raise SystemExit(f"No assets found in {dist_dir}")

    subprocess.run(
        ["gh", "release", "create", tag, *[str(p) for p in assets], "--title", tag, "--generate-notes"],
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

