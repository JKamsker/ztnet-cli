#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess


def _run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Commit Scoop manifest update if changed.")
    parser.add_argument("--tag", required=True)
    parser.add_argument("--branch", default="master")
    args = parser.parse_args()

    diff = subprocess.run(["git", "diff", "--quiet"], check=False)
    if diff.returncode == 0:
        print("No changes to commit")
        return 0

    _run(["git", "add", "bucket/ztnet.json"])
    _run(["git", "commit", "-m", f"chore(scoop): update manifest for {args.tag}"])
    _run(["git", "push", "origin", args.branch])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

