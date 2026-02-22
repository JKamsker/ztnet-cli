#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess


def _run(cmd: list[str]) -> None:
    subprocess.run(cmd, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description="Commit release changes and push an annotated tag.")
    parser.add_argument("--version", required=True)
    parser.add_argument("--branch", default="master")
    args = parser.parse_args()

    tag = f"v{args.version}"

    _run(["git", "add", "Cargo.toml", "Cargo.lock", "bucket/ztnet.json"])

    staged = subprocess.run(["git", "diff", "--cached", "--quiet"], check=False)
    if staged.returncode == 0:
        raise SystemExit("No staged changes; refusing to create release tag.")

    _run(["git", "commit", "-m", f"chore(release): {tag}"])
    _run(["git", "tag", "-a", tag, "-m", tag])

    _run(["git", "push", "origin", args.branch])
    _run(["git", "push", "origin", tag])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

