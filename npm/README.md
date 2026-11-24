# airgap-mcp

Run the [airgap](https://www.useairgap.dev) MCP server with `npx`, no separate install needed.

airgap is an open-source Rust MCP server. It runs your AI coding agent's installs, builds and tests in single-use Linux microVMs on GMI Cloud AgentBox v2 instead of on your computer. It scans untrusted text for prompt injection, audits dependency manifests before install, and redacts secrets from the output your agent reads.

This package is a small launcher. On first run it downloads the prebuilt airgap binary for your platform from [GitHub Releases](https://github.com/dhruvil-codes/airgap/releases), checks its SHA-256 checksum and version, and caches it. Then it starts airgap as a stdio MCP server.

## Use it with an MCP client

```json
{
  "mcpServers": {
    "airgap": {
      "command": "npx",
      "args": ["-y", "airgap-mcp"],
      "env": {
        "GMI_AGENTBOX_API_KEY": "your-agentbox-key",
        "GMI_MAAS_API_KEY": "your-maas-key"
      }
    }
  }
}
```

With no arguments the launcher runs `airgap --mcp`. Any arguments are passed to airgap unchanged, so you can run every airgap command through it:

```sh
npx -y airgap-mcp setup --claude --yes   # register airgap with Claude Code
npx -y airgap-mcp doctor                 # check keys and connectivity
npx -y airgap-mcp --version
```

## Options

| Variable | Effect |
|---|---|
| `AIRGAP_BINARY` | Path to an airgap executable you already have. Skips the download. |
| `AIRGAP_MCP_CACHE` | Where downloaded binaries are kept. Defaults to `%LOCALAPPDATA%\airgap-mcp`, `~/Library/Caches/airgap-mcp` or `$XDG_CACHE_HOME/airgap-mcp` (`~/.cache/airgap-mcp`). |

Supported platforms: Linux, macOS and Windows on x64 or arm64. Node.js 18 or newer.

## Links

- Website and docs: https://www.useairgap.dev
- Docs for agents: https://www.useairgap.dev/llms.txt
- Source: https://github.com/dhruvil-codes/airgap
- MCP Registry: `com.useairgap/airgap`

License: MIT OR Apache-2.0.
