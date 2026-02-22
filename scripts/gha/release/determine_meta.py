#!/usr/bin/env python3
from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path

import tomllib


def _append_github_output(name: str, value: str) -> None:
    github_output = os.environ.get("GITHUB_OUTPUT")
    if not github_output:
        print(f"{name}={value}")
        return
    with Path(github_output).open("a", encoding="utf-8") as f:
        f.write(f"{name}={value}\n")


def main() -> int:
    github_ref = os.environ.get("GITHUB_REF", "")

    if github_ref.startswith("refs/tags/"):
        tag = github_ref.removeprefix("refs/tags/")
        version = tag[1:] if tag.startswith("v") else tag
        mode = "tag"
        tag_out = tag if tag.startswith("v") else f"v{tag}"
    else:
        mode = "auto"

        data = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))
        current = data["package"]["version"]

        match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", current)
        if not match:
            raise SystemExit(f"Unsupported version format: {current!r}")

        major, minor, patch = map(int, match.groups())
        while True:
            patch += 1
            candidate = f"{major}.{minor}.{patch}"
            candidate_tag = f"v{candidate}"
            result = subprocess.run(
                ["git", "rev-parse", "-q", "--verify", f"refs/tags/{candidate_tag}"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )
            if result.returncode != 0:
                version = candidate
                tag_out = candidate_tag
                break

    if not version:
        raise SystemExit("Failed to determine version")

    _append_github_output("mode", mode)
    _append_github_output("version", version)
    _append_github_output("tag", tag_out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
