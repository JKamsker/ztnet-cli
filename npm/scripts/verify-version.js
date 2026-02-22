const fs = require("node:fs");
const path = require("node:path");

const expected = process.env.VERSION;
if (!expected) {
  console.error("[ztnet-cli] VERSION env var is required");
  process.exit(1);
}

const pkgPath = path.join(__dirname, "..", "package.json");
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
const actual = String(pkg?.version ?? "");

if (actual !== String(expected)) {
  console.error(
    `[ztnet-cli] npm/package.json version ${actual} != ${String(expected)}`,
  );
  process.exit(1);
}

