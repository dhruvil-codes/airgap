"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const path = require("node:path");
const { target, assetName, cacheRoot, parseChecksum, argsFor, tarCommand, VERSION } = require("../lib/launcher.js");
const pkg = require("../package.json");

test("maps every supported platform to a published release asset", () => {
  const cases = [
    ["linux", "x64", "airgap-v0.2.0-x86_64-unknown-linux-musl.tar.gz"],
    ["linux", "arm64", "airgap-v0.2.0-aarch64-unknown-linux-musl.tar.gz"],
    ["darwin", "x64", "airgap-v0.2.0-x86_64-apple-darwin.tar.gz"],
    ["darwin", "arm64", "airgap-v0.2.0-aarch64-apple-darwin.tar.gz"],
    ["win32", "x64", "airgap-v0.2.0-x86_64-pc-windows-msvc.zip"],
    ["win32", "arm64", "airgap-v0.2.0-aarch64-pc-windows-msvc.zip"],
  ];
  for (const [platform, arch, name] of cases) assert.equal(assetName("0.2.0", target(platform, arch)), name);
  assert.equal(target("win32", "x64").exe, "airgap.exe");
  assert.equal(target("linux", "x64").exe, "airgap");
});

test("rejects unsupported platforms with a clear message", () => {
  assert.throws(() => target("freebsd", "x64"), /no prebuilt binary for freebsd\/x64/);
  assert.throws(() => target("linux", "ia32"), /no prebuilt binary/);
});

test("parses the release checksum format and refuses anything else", () => {
  const sum = "a".repeat(64);
  const name = "airgap-v0.2.0-x86_64-pc-windows-msvc.zip";
  assert.equal(parseChecksum(`${sum}  ${name}\n`, name), sum);
  assert.equal(parseChecksum(`${sum} *${name}\r\n`, name), sum);
  assert.throws(() => parseChecksum(`${sum}  other.zip\n`, name), /Invalid/);
  assert.throws(() => parseChecksum(`${"A".repeat(64)}  ${name}\n`, name), /Invalid/);
  assert.throws(() => parseChecksum(`${sum}  ${name}\nextra\n`, name), /Invalid/);
});

test("defaults to MCP stdio mode and passes other arguments through", () => {
  assert.deepEqual(argsFor([]), ["--mcp"]);
  assert.deepEqual(argsFor(["setup", "--yes"]), ["setup", "--yes"]);
});

test("caches per platform, honouring overrides", () => {
  const home = path.resolve("/home/u");
  assert.equal(cacheRoot({ AIRGAP_MCP_CACHE: "/tmp/c" }, "linux", home), path.resolve("/tmp/c"));
  assert.equal(cacheRoot({}, "linux", home), path.join(home, ".cache", "airgap-mcp"));
  assert.equal(cacheRoot({ XDG_CACHE_HOME: "/xdg" }, "linux", home), path.join("/xdg", "airgap-mcp"));
  assert.equal(cacheRoot({}, "darwin", home), path.join(home, "Library", "Caches", "airgap-mcp"));
  assert.equal(cacheRoot({ LOCALAPPDATA: "C:\\L" }, "win32", home), path.join("C:\\L", "airgap-mcp"));
});

test("uses Windows' own tar so zip archives extract", () => {
  assert.equal(tarCommand("linux", {}), "tar");
});

test("package metadata satisfies npx and the MCP Registry", () => {
  assert.equal(VERSION, pkg.version);
  assert.equal(pkg.bin["airgap-mcp"], "bin/airgap-mcp.js");
  assert.equal(pkg.mcpName, "com.useairgap/airgap");
  assert.match(pkg.name, /mcp/);
  assert.equal(pkg.homepage, "https://www.useairgap.dev");
  const server = require("../../server.json");
  assert.equal(server.name, pkg.mcpName);
  assert.equal(server.version, pkg.version);
  assert.deepEqual(server.packages.map((p) => [p.registryType, p.identifier, p.version]), [["npm", pkg.name, pkg.version]]);
});
