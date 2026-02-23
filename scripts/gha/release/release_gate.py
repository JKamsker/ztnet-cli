#!/usr/bin/env python3
from __future__ import annotations

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


def _latest_release_tag() -> str | None:
    result = subprocess.run(
        ["git", "describe", "--tags", "--match", "v*", "--abbrev=0"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return None

    tag = result.stdout.strip()
    return tag or None


def _has_src_diff_since(tag: str) -> bool:
    result = subprocess.run(
        ["git", "diff", "--name-only", f"{tag}..HEAD", "--", "src"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=True,
    )
    return bool(result.stdout.strip())


def main() -> int:
    event_name = os.environ.get("GITHUB_EVENT_NAME", "")
    github_ref = os.environ.get("GITHUB_REF", "")

    base_tag = ""

    if event_name == "workflow_dispatch":
        should_run = "true"
        reason = "manual"
    elif github_ref.startswith("refs/tags/"):
        should_run = "true"
        reason = "tag"
    else:
        latest = _latest_release_tag()
        if not latest:
            should_run = "true"
            reason = "no_previous_release_tag"
        else:
            base_tag = latest
            if _has_src_diff_since(latest):
                should_run = "true"
                reason = "src_changed"
            else:
                should_run = "false"
                reason = "no_src_changes"

    _append_github_output("should_run", should_run)
    _append_github_output("reason", reason)
    _append_github_output("base_tag", base_tag)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

