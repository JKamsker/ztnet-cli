#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path


def _parse_sha256_from_file(sha_file: Path) -> str:
    line = sha_file.read_text(encoding="utf-8").strip()
    if not line:
        raise SystemExit(f"Empty sha256 file: {sha_file}")
    return line.split()[0]


def main() -> int:
    parser = argparse.ArgumentParser(description="Update bucket/ztnet.json Scoop manifest.")
    parser.add_argument("--version", required=True)
    parser.add_argument("--sha-file", required=True)
    parser.add_argument("--manifest", default="bucket/ztnet.json")
    parser.add_argument("--cleanup-dir", default=None)
    args = parser.parse_args()

    sha_file = Path(args.sha_file)
    if not sha_file.is_file():
        raise SystemExit(f"Expected sha file not found: {sha_file}")

    hash_ = _parse_sha256_from_file(sha_file)

    manifest = {
        "version": args.version,
        "description": "ZTNet CLI — manage ZeroTier networks via ZTNet",
        "homepage": "https://github.com/JKamsker/ztnet-cli",
        "license": "AGPL-3.0-only",
        "architecture": {
            "64bit": {
                "url": (
                    f"https://github.com/JKamsker/ztnet-cli/releases/download/"
                    f"v{args.version}/ztnet-{args.version}-x86_64-pc-windows-msvc.zip"
                ),
                "hash": hash_,
            }
        },
        "bin": "ztnet.exe",
    }

    Path(args.manifest).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    if args.cleanup_dir:
        shutil.rmtree(args.cleanup_dir, ignore_errors=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

