#!/usr/bin/env python3
from __future__ import annotations

import os
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
    data = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))
    version = data["package"]["version"]
    _append_github_output("version", version)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

