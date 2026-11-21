# Editor setup and diagnostics

`airgap setup` and `airgap init` are aliases. They configure an installed binary and add workspace instructions. They do not call GMI or create compute resources.

## Commands

```sh
airgap setup --yes
airgap init --cursor --yes
airgap setup --claude --yes
airgap setup --vscode --yes
airgap setup --rules-only --yes
airgap doctor
airgap doctor --verbose
```

Without `--yes` or `-y`, setup asks for confirmation in a terminal. Non-interactive runs must pass `--yes`; the installers do this automatically. Editor flags can be combined. Without flags, detection checks workspace/user editor directories and executable names on PATH. If nothing is detected, all three integrations are prepared, including in a clean directory.

## Files and schemas

| Selection | Workspace | User |
| --- | --- | --- |
| Cursor | `.cursor/mcp.json`, under `mcpServers` | `~/.cursor/mcp.json`, under `mcpServers` |
| Claude Code | `.mcp.json`, under `mcpServers` | `~/.claude.json`, under `mcpServers` |
| VS Code | `.vscode/mcp.json`, under `servers`, plus portable `.mcp.json` | None |

Entries are named `airgap`, use stdio, and run the installed executable by absolute path with `--mcp`. The VS Code portable file supports the current Copilot harness; the `servers` file supports editor-managed compatibility configurations. Client versions and trust settings affect which source is active. See the [VS Code reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration), [Cursor guide](https://cursor.com/guides/coding-agent-mcp), and [Claude Code scopes](https://code.claude.com/docs/en/mcp).

Setup updates owned launch fields and discovered environment values. It retains unrelated root keys, other servers, custom airgap fields, and unrelated environment variables. Existing editor credentials survive when no replacement is discovered. Identical entries retain their original bytes and formatting.

When converting an existing airgap entry to stdio, stale remote-only URL, header and OAuth fields are removed so they cannot override the local command.

All files are preflighted before writes. Malformed JSON, invalid object shapes, symlinks and oversized files fail instead of causing a reset. JSON comments and trailing commas are not accepted. Changed files use same-directory temporary files and atomic replacement. A detected concurrent edit aborts that write; close editors that rewrite settings during setup. An I/O failure can leave earlier files applied; rerunning safely completes remaining work.

## Credentials

`Config::discover()` resolves process environment and trusted `.env` / `cloudvet.env` files in the current directory and four parents. Process values win. Provider base URLs are accepted only from the process environment. Dedicated compute and inference keys fall back to `GMI_API_KEY`.

Discovered keys are stored in plaintext only in user-level editor configs (`~/.claude.json`, `~/.cursor/mcp.json`). Workspace files (`.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`) never receive keys, and setup removes keys it finds there. VS Code has no user-level file in setup, so supply keys through the process environment or a `.env` file. Setup prints paths and statuses, never key values. Unix config files written by setup use mode 0600; Windows files inherit directory ACLs. Review diffs before committing. Setup does not stage files or modify ignore rules. Only run it in a trusted workspace.

Non-default model, region and provider URLs are carried into editor configuration. Missing keys produce a warning; set them and rerun setup when available. Local redaction and manifest scanning need no keys.

## Routing instructions

Setup manages a delimited block in `.cursorrules` and `CLAUDE.md`. VS Code setup also updates `.github/copilot-instructions.md`. Text outside that block is preserved. Incomplete or duplicate blocks fail preflight.

`--rules-only` changes only `.cursorrules` and `CLAUDE.md`, without reading editor configuration or discovering keys. Instructions tell agents that "use airgap", or any request to run, test, build or verify code, means they should pick and sequence the Airgap tools themselves: scan external issue or PR text with `airgap_scan_injection`, audit manifests with `airgap_audit_supply` before installing, run commands with `airgap_sandbox_run` on explicit snapshots, verify diffs with `airgap_verify_patch`, and redact logs and diffs with `airgap_redact_secrets`. Failures are reported instead of silently falling back to host execution.

Rules cannot disable other tools, approve a server or guarantee agent behavior. Configure client permissions if remote-only execution is required.

## Doctor

`airgap doctor` concurrently requests the MaaS model catalog and AgentBox compute eligibility, with a three-second timeout per request. It verifies model availability and looks for editor registrations. Failures return a nonzero exit code without raw provider error bodies or credential values.

When every check passes, doctor prints a two-line summary. Failed checks are always printed with a suggested fix. `--verbose` or `-v` lists every check and registered config path. In a terminal, a spinner runs on stderr while the network checks are in flight.

Sub-second completion is not guaranteed. Doctor is read-only: it does not launch a sandbox, run an inference completion, prove isolation or verify client approvals. The known live patch-verification HTTP 403 can still occur after eligibility checks pass.

## Installer options

The installers support x86_64 and ARM64 on Linux, macOS and Windows. They require published release assets and checksums. Linux/macOS require Bash, curl, tar and sha256sum or shasum. Windows uses PowerShell and .NET. No Rust or Python runtime is needed.

- `AIRGAP_VERSION` pins a release version.
- `AIRGAP_INSTALL_DIR` chooses an absolute directory. Defaults use writable `/usr/local/bin`, existing `~/.cargo/bin`, or `~/.local/bin` on Unix; Windows uses `%USERPROFILE%\.cargo\bin`.
- `AIRGAP_NO_PATH_UPDATE=1` skips shell-profile or Windows user-PATH updates for portable installs. Editor entries still use the absolute executable path.

The shell installer appends PATH to the relevant profile when needed, leaving symlinked profiles untouched. Windows updates user PATH when permitted. A new terminal may be needed to invoke `airgap` by name; editors may need a reload regardless of PATH.

Checksums are verified before replacement. Invalid checksums and unexpected executable/archive shapes fail. A checksum provides integrity, not a publisher signature. CI artifacts and drafts are not published releases. See [the release process](releasing.md).
