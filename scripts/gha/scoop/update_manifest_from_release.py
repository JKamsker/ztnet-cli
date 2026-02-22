#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path


def _parse_sha256_from_file(sha_file: Path) -> str:
    line = sha_file.read_text(encoding="utf-8").strip()
    if not line:
        raise SystemExit(f"Empty sha256 file: {sha_file}")
    return line.split()[0]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Download SHA256 from a GitHub release and update bucket/ztnet.json.",
    )
    parser.add_argument("--tag", required=True)
    parser.add_argument("--tmp-dir", default=".tmp_scoop")
    parser.add_argument("--manifest", default="bucket/ztnet.json")
    args = parser.parse_args()

    version = args.tag[1:] if args.tag.startswith("v") else args.tag

    tmp_dir = Path(args.tmp_dir)
    tmp_dir.mkdir(parents=True, exist_ok=True)

    sha_name = f"ztnet-{version}-x86_64-pc-windows-msvc.zip.sha256"
    subprocess.run(
        ["gh", "release", "download", args.tag, "-p", sha_name, "-D", str(tmp_dir)],
        check=True,
    )

    sha_file = tmp_dir / sha_name
    if not sha_file.is_file():
        raise SystemExit(f"Expected {sha_name} not found in {tmp_dir}")

    hash_ = _parse_sha256_from_file(sha_file)

    manifest = {
        "version": version,
        "description": "ZTNet CLI — manage ZeroTier networks via ZTNet",
        "homepage": "https://github.com/JKamsker/ztnet-cli",
        "license": "AGPL-3.0-only",
        "architecture": {
            "64bit": {
                "url": (
                    f"https://github.com/JKamsker/ztnet-cli/releases/download/"
                    f"v{version}/ztnet-{version}-x86_64-pc-windows-msvc.zip"
                ),
                "hash": hash_,
            }
        },
        "bin": "ztnet.exe",
    }

    Path(args.manifest).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    shutil.rmtree(tmp_dir, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

