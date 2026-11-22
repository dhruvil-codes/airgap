# Changelog

Changes are recorded here before each release.

## [0.2.0] - 2026-11-22

### Added

- `airgap_verify_pr`: native 1-shot GitHub PR verification engine with parallel host security gates (`airgap_scan_injection` + manifest supply-chain audit), shallow remote git checkout (`git fetch --depth 1`), auto-build detection, and OpenTUI terminal summary cards.
- Pre-checkout TOCTOU force-push protection verifying `git rev-parse pr-verify` against the audited `head_sha` before running `git checkout --detach` inside the microVM, hardened with `core.hooksPath=/dev/null`, `core.fsmonitor=false`, `submodule.recurse=false`, and `protocol.file.allow=never`.
- Extended test runner parsers for Vitest, Jest, Node built-in test runner (`node --test`), and AVA, with multi-suite aggregation across all summary lines.
- Warm GMI AgentBox template reuse with detail-endpoint fallback lookup (`GET /deployments/{slug}`) and in-process mutex deduplication to prevent duplicate template races.
- Explicit `--claude-bypass-permissions` opt-in flag in `airgap setup` for users who intentionally want `permissions.defaultMode: "bypassPermissions"`.

### Security & Hardening

- **Claude Code permissions scoped by default (C-1)**: `airgap setup` and installers now add only `mcp__airgap__*` to `permissions.allow` and never modify `permissions.defaultMode` unless `--claude-bypass-permissions` is explicitly passed. Workspace MCP configs also strip leaked `GMI_API_KEY`, `GITHUB_TOKEN`, and `GH_TOKEN` entries.
- **Fail-closed PR manifest auditing & pagination (H-1, M-1)**: `airgap_verify_pr` paginates `/files` up to 3 pages (300 files), fails closed if >300 files or >10 manifests are touched, skips only `"removed"` files, percent-encodes manifest path segments with `reqwest::Url`, fails closed on manifest fetch/size/encoding errors, and returns an actionable error on GitHub HTTP 429 or HTTP 403 rate limits (`x-ratelimit-remaining: 0`).
- **Workspace `.env` protection (H-2)**: `.env` and `cloudvet.env` files can no longer override `AIRGAP_NODE_IMAGE`, `GMI_MODEL`, `GMI_AGENTBOX_INSTANCE_TYPE`, or `GMI_DEFAULT_IDC`, and process fallback credentials (`GMI_API_KEY`, `GH_TOKEN`) strictly take precedence over `.env` files.
- **Package-manager & script hardening (H-3, M-2)**: `pnpm install` passes `--ignore-scripts --ignore-pnpmfile`, Yarn exports `YARN_ENABLE_SCRIPTS=false`, high-risk hook/config files (`.pnpmfile.cjs`, `.yarnrc.yml`, `.npmrc`, `.cargo/config.toml`, `setup.py`, `build.rs`) are flagged during PR audits, commands reject NUL (`\0`) bytes, and targeted JS test runners insert `--` before test paths.
- **Immediate cleanup on cancelled MCP calls (H-4)**: `AgentBox::run` and `run_script` wire per-job `CancellationToken` drop guards so cancelled requests immediately abort execution, delete `/tasks/{id}`, and release semaphore permits.

## [0.1.0] - Initial Development

### Added

- `init`/`setup`, `doctor`, help, version and explicit `--mcp` terminal dispatch.
- Idempotent editor merges, managed routing rules, checksum-verified installers and x86_64/ARM64 release targets.
- Seven MCP tools for remote execution, patch verification, injection classification, secret redaction, manifest review, runtime configuration and defensive repository audit.
- GMI AgentBox and MaaS clients with bounded requests, explicit sandbox lifetimes and awaited cleanup.
- Environment-file discovery, snapshot validation and unsigned SHA-256 evidence receipts.
- Offline regression tests, real stdio protocol checks and documented live smoke-check results.
- Contributor guidance, security policy, GitHub issue and PR templates, cross-platform CI and release-candidate packaging.

### Changed

- GMI AgentBox deployment templates are cached and reused. Each run looks up a ready template named `airgap-{image}-{instance_type}` with `GET /deployments` and creates one only when none is ready. Cleanup deletes only the sandbox task, so warm runs skip the multi-minute template build (about 98% less startup latency on a warm template).
- `airgap_sandbox_run`, `airgap_verify_patch` and `airgap_audit_supply` render box-drawn terminal cards (`╭─`, `│`, `├─`, `╰─`) with status badges, compute and image fields, and a pass-rate progress bar for patch verification. Command output is stripped of ANSI escapes and control characters and wrapped to the card width. Secret redaction and injection scans keep their compact Markdown cards.
- MCP tool text output for secret redaction and injection scans is compact Markdown (verdict, duration and host-protection note) instead of ASCII banners and box frames; sandbox, patch verification and supply-chain output use the box-drawn cards described above. `structuredContent` is unchanged; ASCII art remains only in terminal commands.
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

- A terminal AgentBox execution with a null `exit_code` (timed out, cancelled or errored) no longer fails the tool call with `terminal execution has no exit code`. It reports exit code 124 (`timed_out`), 130 (`cancelled`/`canceled`) or -1, and is never marked passed. Out-of-range exit codes also map to -1.
- `airgap_sandbox_run` no longer reports PASSED when output shows a `Killed`/`SIGKILL` termination or a recognized test summary reports failures, even with exit code 0. Exit code 137 gets an out-of-memory remedy.
- Normalized Windows path separators in `setup` and `doctor` output to eliminate mixed forward/backward slashes.
- Base URLs (`GMI_AGENTBOX_BASE_URL`, `GMI_MAAS_BASE_URL`) are read exclusively from process environment variables; repository `.env` values are ignored to prevent malicious repos from exfiltrating credentials.
- Workspace configuration files (`.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`) no longer receive API keys during setup; keys are restricted to user-level configs (`~/.claude.json`, `~/.cursor/mcp.json`).
- Untrusted text in `airgap_scan_injection` is scrubbed with the local secret redactor before uploading to GMI MaaS.
- Secret redaction avoids false positives on configuration parameters like `max_tokens = 4096` and `tokenizer: gpt2` by enforcing boundary delimiters on key names and skipping short or digit-only values for general keys.
- Explicit `password` and `passwd` keys are always redacted regardless of length, preserving coverage for short passwords while leaving harmless numeric settings untouched.
- Sandbox summary verdict requires provider `passed == true` and `exit_code == 0`, preventing false "PASSED" reports on timed-out or cancelled runs.
- Repository audit verifies that serialized snapshot size is within the 2 MiB limit before starting any billable AgentBox inventory microVM.
- AgentBox cleanup failures keep successful execution outputs and add a `cleanup_warning`.
- Verify summary reports both zero passed and nonzero failed test counts concurrently.

### Known limitations

- Live patch verification receives HTTP 403 at the AgentBox execution endpoint on the tested account.
- Model output can be incorrect or malformed; malformed output fails closed.
- Live provider checks have only been run from Windows. Measured Windows latency exceeds the original targets.
