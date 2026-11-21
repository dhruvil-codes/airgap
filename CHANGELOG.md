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

### Fixed

- Clean injection results require an empty threat findings array; the prompt no longer asks for reassuring explanations in that array.
- Repository audit instructions distinguish concrete security impact from ordinary type and length hardening advice.
- Provider errors identify the operation without including resource identifiers or raw response bodies.

### Known limitations

- Live patch verification receives HTTP 403 at the AgentBox execution endpoint on the tested account.
- Model output can be incorrect or malformed; malformed output fails closed.
- Live provider checks have only been run from Windows. Measured Windows latency exceeds the original targets.
