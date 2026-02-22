const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const extractZip = require("extract-zip");
const tar = require("tar");

function resolvePlatformTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "win32" && arch === "x64") {
    return { target: "x86_64-pc-windows-msvc", archiveExt: "zip", exe: "ztnet.exe" };
  }

  if (platform === "linux" && arch === "x64") {
    return { target: "x86_64-unknown-linux-gnu", archiveExt: "tar.gz", exe: "ztnet" };
  }

  if (platform === "darwin" && arch === "arm64") {
    return { target: "aarch64-apple-darwin", archiveExt: "tar.gz", exe: "ztnet" };
  }

  if (platform === "darwin" && arch === "x64") {
    return { target: "x86_64-apple-darwin", archiveExt: "tar.gz", exe: "ztnet" };
  }

  return null;
}

function readPackageJsonVersion(packageRoot) {
  const pkgJsonPath = path.join(packageRoot, "package.json");
  const pkg = JSON.parse(fs.readFileSync(pkgJsonPath, "utf8"));
  if (!pkg?.version) throw new Error("Failed to read version from package.json");
  return String(pkg.version);
}

function httpGet(url) {
  return new Promise((resolve, reject) => {
    https
      .get(
        url,
        {
          headers: {
            "User-Agent": "ztnet-cli-npm-installer",
            Accept: "application/octet-stream",
          },
        },
        (res) => {
          const code = res.statusCode ?? 0;

          if (code >= 300 && code < 400 && res.headers.location) {
            res.resume();
            resolve(httpGet(res.headers.location));
            return;
          }

          if (code !== 200) {
            const chunks = [];
            res.on("data", (c) => chunks.push(c));
            res.on("end", () => {
              const body = Buffer.concat(chunks).toString("utf8").slice(0, 3000);
              reject(new Error(`GET ${url} failed: HTTP ${code}\n${body}`));
            });
            return;
          }

          const chunks = [];
          res.on("data", (c) => chunks.push(c));
          res.on("end", () => resolve(Buffer.concat(chunks)));
          res.on("error", reject);
        },
      )
      .on("error", reject);
  });
}

function downloadToFile(url, outPath) {
  return new Promise((resolve, reject) => {
    https
      .get(
        url,
        {
          headers: {
            "User-Agent": "ztnet-cli-npm-installer",
            Accept: "application/octet-stream",
          },
        },
        (res) => {
          const code = res.statusCode ?? 0;

          if (code >= 300 && code < 400 && res.headers.location) {
            res.resume();
            resolve(downloadToFile(res.headers.location, outPath));
            return;
          }

          if (code !== 200) {
            const chunks = [];
            res.on("data", (c) => chunks.push(c));
            res.on("end", () => {
              const body = Buffer.concat(chunks).toString("utf8").slice(0, 3000);
              reject(new Error(`Download ${url} failed: HTTP ${code}\n${body}`));
            });
            return;
          }

          const file = fs.createWriteStream(outPath);
          res.pipe(file);

          file.on("finish", () => file.close(resolve));
          file.on("error", (err) => {
            try {
              fs.unlinkSync(outPath);
            } catch {
              // ignore
            }
            reject(err);
          });
        },
      )
      .on("error", reject);
  });
}

function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash("sha256");
    const stream = fs.createReadStream(filePath);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("error", reject);
    stream.on("end", () => resolve(hash.digest("hex")));
  });
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function safeCopyFile(src, dest) {
  ensureDir(path.dirname(dest));
  fs.copyFileSync(src, dest);
}

async function main() {
  const packageRoot = path.join(__dirname, "..");
  const resolved = resolvePlatformTarget();

  if (!resolved) {
    console.error(
      `[ztnet-cli] Unsupported platform/arch: ${process.platform}/${process.arch}`,
    );
    process.exit(1);
  }

  const version = readPackageJsonVersion(packageRoot);
  const repo = process.env.ZTNET_CLI_GITHUB_REPO || "JKamsker/ztnet-cli";
  const tag = `v${version}`;

  const asset = `ztnet-${version}-${resolved.target}.${resolved.archiveExt}`;
  const assetUrl = `https://github.com/${repo}/releases/download/${tag}/${asset}`;
  const shaUrl = `${assetUrl}.sha256`;
  const localArtifactsDir = path.join(packageRoot, "artifacts");
  const localArchivePath = path.join(localArtifactsDir, asset);
  const localShaPath = `${localArchivePath}.sha256`;

  const vendorDir = path.join(packageRoot, "vendor");
  ensureDir(vendorDir);
  const destBinary = path.join(vendorDir, resolved.exe);

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ztnet-cli-"));
  const archivePath = path.join(tmpDir, asset);
  const extractDir = path.join(tmpDir, "extract");
  ensureDir(extractDir);

  try {
    const usingBundledAsset =
      fs.existsSync(localArchivePath) && fs.existsSync(localShaPath);

    const shaText = usingBundledAsset
      ? fs.readFileSync(localShaPath, "utf8").trim()
      : (await httpGet(shaUrl)).toString("utf8").trim();
    const expectedHash = shaText.split(/\s+/)[0]?.trim();
    if (!expectedHash || !/^[0-9a-fA-F]{64}$/.test(expectedHash)) {
      const source = usingBundledAsset ? localShaPath : shaUrl;
      throw new Error(`Invalid SHA256 file at ${source}`);
    }

    const actualArchivePath = usingBundledAsset ? localArchivePath : archivePath;

    if (!usingBundledAsset) {
      await downloadToFile(assetUrl, archivePath);
    }

    const actualHash = await sha256File(actualArchivePath);
    if (actualHash.toLowerCase() !== expectedHash.toLowerCase()) {
      throw new Error(
        `SHA256 mismatch for ${asset}\nExpected: ${expectedHash}\nActual:   ${actualHash}`,
      );
    }

    if (resolved.archiveExt === "zip") {
      await extractZip(actualArchivePath, { dir: extractDir });
    } else if (resolved.archiveExt === "tar.gz") {
      await tar.x({ file: actualArchivePath, cwd: extractDir });
    } else {
      throw new Error(`Unsupported archive type: ${resolved.archiveExt}`);
    }

    const extractedBinary = path.join(extractDir, resolved.exe);
    if (!fs.existsSync(extractedBinary)) {
      throw new Error(`Expected extracted binary not found: ${extractedBinary}`);
    }

    safeCopyFile(extractedBinary, destBinary);

    if (process.platform !== "win32") {
      fs.chmodSync(destBinary, 0o755);
    }
  } finally {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // ignore
    }
  }
}

main().catch((err) => {
  console.error(`[ztnet-cli] Install failed: ${err?.message || String(err)}`);
  process.exit(1);
});

