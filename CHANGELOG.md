# Changelog

Changes are recorded here before a release. Version 0.1.0 in Cargo.toml identifies the initial development version; it is not evidence of a published release.

## Unreleased

### Added

- `init`/`setup`, `doctor`, help, version and explicit `--mcp` terminal dispatch.
- Idempotent editor merges, managed routing rules, checksum-verified installers and x86_64/ARM64 release targets.
- Seven MCP tools for remote execution, patch verification, injection classification, secret redaction, manifest review, runtime configuration and defensive repository audit.
- GMI AgentBox and MaaS clients with bounded requests, explicit sandbox lifetimes and awaited cleanup.
- Environment-file discovery, snapshot validation and unsigned SHA-256 evidence receipts.
- Offline regression tests, real stdio protocol checks and documented live smoke-check results.
- Contributor guidance, security policy, GitHub issue and PR templates, cross-platform CI and release-candidate packaging.

### Changed

- MCP tool text output for sandbox runs, patch verification, supply-chain audit, secret redaction and injection scans is compact Markdown (verdict, duration, environment and host-protection note) instead of ASCII banners and box frames. `structuredContent` is unchanged; ASCII art remains only in terminal commands.
- Autonomous agent routing rules and trigger descriptions guide coding agents to sequence tools automatically upon request ("use airgap", "run tests", "fix issue"), with autonomous fallback to `airgap_sandbox_run` when patch verification returns a provider error.
- `airgap doctor` displays registered editor configurations with explicit client and scope labels (`Claude (user)`, `Cursor (project)`, etc.) and high-contrast terminal indicators.
- Deduplicated terminal ASCII art banners across `main`, `setup`, and `doctor` into a unified `airgap::banner` helper.
- The terminal header renders "AIRGAP" in OpenTUI's two-row `tiny` font, with a cyan top row and an indigo bottom row (24-bit color), above a version and subtitle line.
- `airgap doctor` shows a spinner while network checks run, prints a two-line summary when everything passes, and expands only failed checks with a remedy. `airgap doctor --verbose` (`-v`) lists every check and config path.
- Setup configures Factory Droid (`.factory/mcp.json` in the workspace and `~/.factory/mcp.json`, `--factory`/`--droid`) and writes the routing block to `AGENTS.md` as well as `.cursorrules` and `CLAUDE.md`.
- Claude Code setup adds `mcp__airgap__*` to `permissions.allow` in `~/.claude/settings.json` (and `.claude/settings.json` when present), so airgap tools no longer prompt on every call.
- Sandboxes default to `gmi.sandbox.medium` (4 GiB) instead of `gmi.sandbox.small` (2 GiB); override with `GMI_AGENTBOX_INSTANCE_TYPE`.
- Sandbox commands use `TMPDIR=/root/tmp` on the root disk instead of the 1 GiB `/tmp` tmpfs, enable Corepack when available, and put `PNPM_HOME` on `PATH`.
- Upgraded `README.md` to tier-1 open source documentation standards with native Mermaid architecture modeling, host-versus-microVM comparison matrix, and production output cards.

### Fixed

- `airgap_sandbox_run` no longer reports PASSED when output shows a `Killed`/`SIGKILL` termination or a recognized test summary reports failures, even with exit code 0. Exit code 137 gets an out-of-memory remedy.
- Normalized Windows path separators in `setup` and `doctor` output to eliminate mixed forward/backward slashes.
- Base URLs (`GMI_AGENTBOX_BASE_URL`, `GMI_MAAS_BASE_URL`) are read exclusively from process environment variables; repository `.env` values are ignored to prevent malicious repos from exfiltrating credentials.
- Workspace configuration files (`.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`) no longer receive API keys during setup; keys are restricted to user-level configs (`~/.claude.json`, `~/.cursor/mcp.json`).
- Untrusted text in `airgap_scan_injection` is scrubbed with the local secret redactor before uploading to GMI MaaS.
- Secret redaction avoids false positives on configuration parameters like `max_tokens = 4096` and `tokenizer: gpt2` by enforcing boundary delimiters on key names and skipping short or digit-only values for general keys.
- Explicit `password` and `passwd` keys are always redacted regardless of length, preserving coverage for short passwords while leaving harmless numeric settings untouched.
- Sandbox summary verdict requires provider `passed == true` and `exit_code == 0`, preventing false "PASSED" reports on timed-out or cancelled runs.
- Repository audit verifies that serialized snapshot size is within the 2 MiB limit before starting any billable AgentBox inventory microVM.
- AgentBox cleanup deletes tasks and deployment templates independently; successful execution outputs are retained with a `cleanup_warning` if remote teardown fails.
- Verify summary reports both zero passed and nonzero failed test counts concurrently.

### Known limitations

- Live patch verification receives HTTP 403 at the AgentBox execution endpoint on the tested account.
- Model output can be incorrect or malformed; malformed output fails closed.
- Live provider checks have only been run from Windows. Measured Windows latency exceeds the original targets.
