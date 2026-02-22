#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import subprocess
from pathlib import Path


def _append_github_output(name: str, value: str) -> None:
    github_output = os.environ.get("GITHUB_OUTPUT")
    if not github_output:
        print(f"{name}={value}")
        return
    with Path(github_output).open("a", encoding="utf-8") as f:
        f.write(f"{name}={value}\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Check whether a GitHub release exists for a tag.")
    parser.add_argument("--version", required=True)
    parser.add_argument("--output-name", default="exists")
    args = parser.parse_args()

    tag = f"v{args.version}"
    result = subprocess.run(
        ["gh", "release", "view", tag],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )

    _append_github_output(args.output_name, "true" if result.returncode == 0 else "false")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

