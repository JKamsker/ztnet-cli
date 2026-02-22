#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess


def main() -> int:
    parser = argparse.ArgumentParser(description="Configure git user for GitHub Actions.")
    parser.add_argument("--name", default="github-actions[bot]")
    parser.add_argument(
        "--email",
        default="41898282+github-actions[bot]@users.noreply.github.com",
    )
    args = parser.parse_args()

    subprocess.run(["git", "config", "user.name", args.name], check=True)
    subprocess.run(["git", "config", "user.email", args.email], check=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

