#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


def _update_cargo_toml_version(cargo_toml: Path, new_version: str) -> None:
    lines = cargo_toml.read_text(encoding="utf-8").splitlines()

    in_package = False
    updated = False
    for i, line in enumerate(lines):
        if line.strip() == "[package]":
            in_package = True
            continue
        if in_package and line.lstrip().startswith("[") and line.strip() != "[package]":
            in_package = False
        if not in_package:
            continue

        match = re.match(r"^(\s*)version\s*=", line)
        if match:
            indent = match.group(1)
            lines[i] = f'{indent}version = "{new_version}"'
            updated = True
            break

    if not updated:
        raise SystemExit("Could not update [package].version in Cargo.toml")

    cargo_toml.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _update_cargo_lock_root_version(cargo_lock: Path, crate_name: str, new_version: str) -> None:
    if not cargo_lock.is_file():
        return

    lines = cargo_lock.read_text(encoding="utf-8").splitlines()

    in_package = False
    is_target_package = False
    updated = False

    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped == "[[package]]":
            in_package = True
            is_target_package = False
            continue
        if in_package and stripped == "":
            in_package = False
            is_target_package = False
            continue
        if not in_package:
            continue

        if stripped == f'name = "{crate_name}"':
            is_target_package = True
            continue

        if is_target_package and stripped.startswith("version ="):
            indent = line[: len(line) - len(line.lstrip(" "))]
            lines[i] = f'{indent}version = "{new_version}"'
            updated = True
            break

    if not updated:
        raise SystemExit(f"Could not find version for {crate_name} in Cargo.lock")

    cargo_lock.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _update_npm_package_json_version(npm_package_json: Path, new_version: str) -> None:
    if not npm_package_json.is_file():
        return

    data = json.loads(npm_package_json.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise SystemExit(f"Unsupported npm package.json format: {npm_package_json}")

    data["version"] = new_version
    npm_package_json.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="Apply a version bump to Cargo.toml/Cargo.lock.")
    parser.add_argument("--version", required=True)
    parser.add_argument("--crate-name", default="ztnet")
    parser.add_argument("--cargo-toml", default="Cargo.toml")
    parser.add_argument("--cargo-lock", default="Cargo.lock")
    parser.add_argument("--npm-package-json", default="npm/package.json")
    args = parser.parse_args()

    _update_cargo_toml_version(Path(args.cargo_toml), args.version)
    _update_cargo_lock_root_version(Path(args.cargo_lock), args.crate_name, args.version)
    _update_npm_package_json_version(Path(args.npm_package_json), args.version)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
