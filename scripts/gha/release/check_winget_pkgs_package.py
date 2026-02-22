#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path


def _append_github_output(name: str, value: str) -> None:
    github_output = os.environ.get("GITHUB_OUTPUT")
    if not github_output:
        print(f"{name}={value}")
        return
    with Path(github_output).open("a", encoding="utf-8") as f:
        f.write(f"{name}={value}\n")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Checks whether winget-pkgs already contains the ZTNetCLI package.",
    )
    parser.add_argument(
        "--output-name",
        default="exists",
        help="Output key name.",
    )
    args = parser.parse_args()

    url = (
        "https://api.github.com/repos/microsoft/winget-pkgs/contents/"
        "manifests/j/JKamsker/ZTNetCLI?ref=master"
    )
    req = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})

    try:
        with urllib.request.urlopen(req) as resp:  # noqa: S310
            status = getattr(resp, "status", 200)
    except urllib.error.HTTPError as e:
        status = e.code
    except Exception as e:  # pragma: no cover
        print(f"Failed to query winget-pkgs: {e}", file=sys.stderr)
        _append_github_output(args.output_name, "false")
        return 0

    if status == 200:
        _append_github_output(args.output_name, "true")
        return 0

    print(f"Package not in winget-pkgs yet (HTTP {status}); skipping")
    _append_github_output(args.output_name, "false")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

