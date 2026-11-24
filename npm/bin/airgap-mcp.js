#!/usr/bin/env node
"use strict";
const { spawn } = require("node:child_process");
const { argsFor, install } = require("../lib/launcher.js");

async function main() {
  // AIRGAP_BINARY points at an existing airgap executable and skips the download.
  const exe = process.env.AIRGAP_BINARY || (await install());
  const child = spawn(exe, argsFor(process.argv.slice(2)), { stdio: "inherit", windowsHide: true });
  for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) process.on(sig, () => child.kill(sig));
  child.on("error", (err) => {
    process.stderr.write(`airgap-mcp: could not start ${exe}: ${err.message}\n`);
    process.exit(1);
  });
  child.on("exit", (code, signal) => {
    if (signal) process.kill(process.pid, signal);
    else process.exit(code ?? 1);
  });
}

main().catch((err) => {
  process.stderr.write(`airgap-mcp: ${err.message}\n`);
  process.exit(1);
});
