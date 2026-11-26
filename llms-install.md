# Installing airgap MCP Server

This guide is for AI coding agents (such as Cline, Claude Code, Cursor, VS Code, and Factory Droid) performing automated setup of the `airgap` MCP server.

## Prerequisites

- **Node.js 18+** (recommended; `npx -y airgap-mcp` automatically downloads, SHA-256 verifies, and caches the prebuilt Rust binary for macOS, Linux, or Windows).
- **GMI Cloud API keys** (optional for local scanners; required for remote microVM sandboxes and prompt-injection classification):
  - `GMI_AGENTBOX_API_KEY` — compute-scoped key for GMI AgentBox v2 microVMs (`airgap_sandbox_run`, `airgap_verify_patch`, `airgap_verify_pr`, `airgap_audit_repo`).
  - `GMI_MAAS_API_KEY` — inference key for GMI MaaS (`airgap_scan_injection`, `airgap_audit_repo`).
  - `GMI_API_KEY` — optional single-key fallback if your key has both scopes.

Local tools (`airgap_redact_secrets`, `airgap_audit_supply`, `airgap_configure`) work immediately with no API keys.

## Step 1: Collect optional credentials

Ask the user whether they want to configure GMI Cloud API keys now or start in local-only mode:
- If they have keys, ask for `GMI_AGENTBOX_API_KEY` and `GMI_MAAS_API_KEY` (or a single `GMI_API_KEY`).
- Never write API keys into git-tracked workspace files (`.mcp.json`, `.vscode/mcp.json`, etc.). Store them only in the user-level MCP client settings file.

## Step 2: Add airgap to the MCP client configuration

Add the following entry under `mcpServers` in the user's MCP settings JSON (for Cline, `cline_mcp_settings.json`):

```json
{
  "mcpServers": {
    "airgap": {
      "command": "npx",
      "args": ["-y", "airgap-mcp"],
      "env": {
        "GMI_AGENTBOX_API_KEY": "<YOUR_GMI_AGENTBOX_API_KEY>",
        "GMI_MAAS_API_KEY": "<YOUR_GMI_MAAS_API_KEY>"
      }
    }
  }
}
```

If starting without API keys (local scanners only), omit the `env` block:

```json
{
  "mcpServers": {
    "airgap": {
      "command": "npx",
      "args": ["-y", "airgap-mcp"]
    }
  }
}
```

### Alternative: Native binary (no Node.js required)

If Node.js is not available, install the prebuilt binary directly and set `"command": "airgap"` with `"args": []`:

- **macOS / Linux:**
  ```sh
  curl -fsSL https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts/install.sh | bash
  ```
- **Windows PowerShell:**
  ```powershell
  irm https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts/install.ps1 | iex
  ```

## Step 3: Verify the installation

1. Confirm the `airgap` server connects over `stdio` and exposes its tools (`airgap_sandbox_run`, `airgap_verify_patch`, `airgap_verify_pr`, `airgap_scan_injection`, `airgap_audit_supply`, `airgap_redact_secrets`, `airgap_audit_repo`, `airgap_configure`).
2. Run a quick zero-cost local smoke test by calling `airgap_redact_secrets` with `"TOKEN=sk-test-1234567890abcdef"` and confirming the secret is redacted.
3. If GMI keys were configured, optionally run `npx -y airgap-mcp doctor` to verify API connectivity without creating billable resources.
