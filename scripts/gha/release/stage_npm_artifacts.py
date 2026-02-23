#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shutil
from pathlib import Path


def _copy(src: Path, dest_dir: Path) -> None:
    dest_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest_dir / src.name)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Stage built release archives into npm/artifacts for npm publish.",
    )
    parser.add_argument("--version", required=True)
    parser.add_argument("--bin-name", default="ztnet")
    parser.add_argument("--dist-dir", default="dist")
    parser.add_argument("--npm-dir", default="npm")
    args = parser.parse_args()

    dist_dir = Path(args.dist_dir)
    if not dist_dir.is_dir():
        raise SystemExit(f"dist dir not found: {dist_dir}")

    npm_dir = Path(args.npm_dir)
    if not npm_dir.is_dir():
        raise SystemExit(f"npm dir not found: {npm_dir}")

    artifacts_dir = npm_dir / "artifacts"
    if artifacts_dir.exists():
        shutil.rmtree(artifacts_dir)
    artifacts_dir.mkdir(parents=True, exist_ok=True)

    patterns = [
        f"{args.bin_name}-{args.version}-*.zip",
        f"{args.bin_name}-{args.version}-*.tar.gz",
    ]
    archives: list[Path] = []
    for pattern in patterns:
        archives.extend(sorted(dist_dir.glob(pattern)))

    if not archives:
        raise SystemExit(f"No archives found in {dist_dir} for version {args.version}")

    staged_count = 0
    for archive in archives:
        sha = dist_dir / f"{archive.name}.sha256"
        if not sha.is_file():
            raise SystemExit(f"Missing sha256 file for {archive.name}: {sha}")

        _copy(archive, artifacts_dir)
        _copy(sha, artifacts_dir)
        staged_count += 2

    print(f"Staged {staged_count} files into {artifacts_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

