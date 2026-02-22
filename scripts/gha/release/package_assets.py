#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import os
import subprocess
import shutil
import tarfile
import zipfile
from pathlib import Path


def _detect_rust_host() -> str:
    rustc = subprocess.run(
        ["rustc", "-vV"],
        capture_output=True,
        text=True,
        check=True,
    )
    for line in rustc.stdout.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise SystemExit("Failed to determine rust host target triple from `rustc -vV`")


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Package built binaries into release assets + sha256.")
    parser.add_argument("--version", required=True)
    parser.add_argument("--bin-name", default="ztnet")
    parser.add_argument("--dist-dir", default="dist")
    parser.add_argument("--release-dir", default="target/release")
    args = parser.parse_args()

    runner_os = (os.environ.get("RUNNER_OS") or "").lower()
    is_windows = runner_os == "windows"

    target = _detect_rust_host()
    dist_dir = Path(args.dist_dir)
    dist_dir.mkdir(parents=True, exist_ok=True)

    binary_name = f"{args.bin_name}.exe" if is_windows else args.bin_name
    src_binary = Path(args.release_dir) / binary_name
    if not src_binary.is_file():
        raise SystemExit(f"Expected binary not found: {src_binary}")

    dst_binary = dist_dir / binary_name
    shutil.copy2(src_binary, dst_binary)

    if is_windows:
        asset_name = f"{args.bin_name}-{args.version}-{target}.zip"
        asset_path = dist_dir / asset_name
        with zipfile.ZipFile(asset_path, mode="w", compression=zipfile.ZIP_DEFLATED) as z:
            z.write(dst_binary, arcname=binary_name)
    else:
        asset_name = f"{args.bin_name}-{args.version}-{target}.tar.gz"
        asset_path = dist_dir / asset_name
        with tarfile.open(asset_path, mode="w:gz") as t:
            t.add(dst_binary, arcname=binary_name)

    sha = _sha256_file(asset_path)
    (dist_dir / f"{asset_name}.sha256").write_text(
        f"{sha}  {asset_name}\n",
        encoding="utf-8",
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
