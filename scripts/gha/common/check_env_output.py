#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
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
        description="Writes a boolean output based on whether an env var is set.",
    )
    parser.add_argument("--env-var", required=True, help="Environment variable to check.")
    parser.add_argument("--output-name", default="present", help="Output key name.")
    parser.add_argument(
        "--missing-message",
        default=None,
        help="Message to print when missing (optional).",
    )
    args = parser.parse_args()

    raw_value = os.environ.get(args.env_var, "")
    present = bool(raw_value.strip())
    _append_github_output(args.output_name, "true" if present else "false")

    if not present and args.missing_message:
        print(args.missing_message)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

