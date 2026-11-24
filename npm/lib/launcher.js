"use strict";
// Downloads, verifies and caches the airgap release binary that matches this package's version.
// Everything this module prints goes to stderr: stdout belongs to the MCP stdio protocol.
const { createHash } = require("node:crypto");
const { execFileSync, spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const REPO = "https://github.com/dhruvil-codes/airgap/releases/download";
const VERSION = require("../package.json").version;
const MAX_ARCHIVE_BYTES = 256 * 1024 * 1024;

function target(platform = process.platform, arch = process.arch) {
  const cpu = { x64: "x86_64", arm64: "aarch64" }[arch];
  const os_ = { linux: "unknown-linux-musl", darwin: "apple-darwin", win32: "pc-windows-msvc" }[platform];
  if (!cpu || !os_) throw new Error(`airgap has no prebuilt binary for ${platform}/${arch}. Supported: Linux, macOS and Windows on x64 or arm64.`);
  const windows = platform === "win32";
  return { triple: `${cpu}-${os_}`, ext: windows ? "zip" : "tar.gz", exe: windows ? "airgap.exe" : "airgap" };
}

function assetName(version, t) {
  return `airgap-v${version}-${t.triple}.${t.ext}`;
}

function cacheRoot(env = process.env, platform = process.platform, home = os.homedir()) {
  if (env.AIRGAP_MCP_CACHE) return path.resolve(env.AIRGAP_MCP_CACHE);
  if (platform === "win32") return path.join(env.LOCALAPPDATA || path.join(home, "AppData", "Local"), "airgap-mcp");
  if (platform === "darwin") return path.join(home, "Library", "Caches", "airgap-mcp");
  return path.join(env.XDG_CACHE_HOME || path.join(home, ".cache"), "airgap-mcp");
}

// Same format the release workflow writes and scripts/install.* accept: "<sha256>  <asset name>".
function parseChecksum(text, expectedName) {
  const m = /^([a-f0-9]{64})[ \t]+\*?([^\r\n]+)\r?\n?$/.exec(text);
  if (!m || m[2] !== expectedName) throw new Error("Invalid release checksum file.");
  return m[1];
}

function argsFor(argv) {
  return argv.length ? argv : ["--mcp"];
}

function log(message) {
  process.stderr.write(`airgap-mcp: ${message}\n`);
}

async function download(url) {
  const res = await fetch(url, { redirect: "follow", headers: { "user-agent": `airgap-mcp/${VERSION}` } });
  if (!res.ok) throw new Error(`Download failed (${res.status}) for ${url}`);
  const length = Number(res.headers.get("content-length") || 0);
  if (length > MAX_ARCHIVE_BYTES) throw new Error("Release archive is unexpectedly large.");
  const buf = Buffer.from(await res.arrayBuffer());
  if (buf.length > MAX_ARCHIVE_BYTES) throw new Error("Release archive is unexpectedly large.");
  return buf;
}

// Windows' own bsdtar reads zip archives; a GNU tar earlier on PATH (Git for Windows) cannot.
function tarCommand(platform = process.platform, env = process.env) {
  if (platform === "win32") {
    const system = path.join(env.SystemRoot || "C:\\Windows", "System32", "tar.exe");
    if (fs.existsSync(system)) return system;
  }
  return "tar";
}

function binaryVersion(file) {
  const out = spawnSync(file, ["--version"], { encoding: "utf8", windowsHide: true, timeout: 30000 });
  return (out.stdout || "").trim();
}

async function install(version = VERSION, env = process.env) {
  const t = target();
  const dir = path.join(cacheRoot(env), `v${version}`, t.triple);
  const exe = path.join(dir, t.exe);
  if (fs.existsSync(exe) && binaryVersion(exe) === `airgap ${version}`) return exe;

  const name = assetName(version, t);
  log(`downloading ${name}`);
  const [archive, sums] = await Promise.all([download(`${REPO}/v${version}/${name}`), download(`${REPO}/v${version}/${name}.sha256`)]);
  const expected = parseChecksum(sums.toString("utf8"), name);
  const actual = createHash("sha256").update(archive).digest("hex");
  if (actual !== expected) throw new Error("Checksum mismatch; nothing was installed.");

  fs.mkdirSync(dir, { recursive: true });
  const work = fs.mkdtempSync(path.join(dir, ".tmp-"));
  try {
    const archivePath = path.join(work, name);
    fs.writeFileSync(archivePath, archive);
    execFileSync(tarCommand(), ["-xf", archivePath, "-C", work, t.exe], { stdio: ["ignore", "ignore", "pipe"], windowsHide: true });
    const extracted = path.join(work, t.exe);
    const st = fs.lstatSync(extracted);
    if (!st.isFile()) throw new Error("Archive must contain one regular airgap executable.");
    if (process.platform !== "win32") fs.chmodSync(extracted, 0o755);
    const got = binaryVersion(extracted);
    if (got !== `airgap ${version}`) throw new Error(`Downloaded executable reports "${got}", expected "airgap ${version}".`);
    fs.renameSync(extracted, exe);
  } finally {
    fs.rmSync(work, { recursive: true, force: true });
  }
  log(`installed airgap ${version} to ${exe}`);
  return exe;
}

module.exports = { VERSION, target, assetName, cacheRoot, parseChecksum, argsFor, tarCommand, install };
