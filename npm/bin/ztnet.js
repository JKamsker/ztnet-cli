#!/usr/bin/env node

const childProcess = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const packageRoot = path.join(__dirname, "..");
const vendorDir = path.join(packageRoot, "vendor");
const exeName = process.platform === "win32" ? "ztnet.exe" : "ztnet";
const exePath = path.join(vendorDir, exeName);

if (!fs.existsSync(exePath)) {
  console.error(`[ztnet-cli] Missing native binary: ${exePath}`);
  console.error(
    "[ztnet-cli] Reinstall to re-run the downloader: npm install -g ztnet-cli",
  );
  process.exit(1);
}

const result = childProcess.spawnSync(exePath, process.argv.slice(2), {
  stdio: "inherit",
});

if (result.error) {
  console.error(`[ztnet-cli] Failed to run ${exePath}: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);

