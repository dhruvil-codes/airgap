# airgap

[![CI](https://github.com/dhruvil-codes/airgap/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/dhruvil-codes/airgap/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.89%2B-orange?logo=rust)](https://www.rust-lang.org/tools/install)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**Remote execution and defensive security tools for AI coding agents.**

airgap is a Rust [Model Context Protocol](https://modelcontextprotocol.io/) server. It sends explicitly supplied files and commands to disposable GMI AgentBox sandboxes and exposes local scanners and model-assisted security review over stdio. Submitted commands never execute on the machine running the server.

> **Early development.** Local tests and a native Windows release build are validated. Live sandbox execution and inference work with the tested account. Patch verification currently receives HTTP 403 from GMI's execution endpoint. See the [validation record](docs/validation.md) before relying on the results.

## What it does

| Tool | Purpose | Runs in |
| --- | --- | --- |
| `airgap_sandbox_run` | Build, test, or execute commands against explicit file snapshots | GMI AgentBox |
| `airgap_verify_patch` | Apply a diff to a base snapshot and run tests | GMI AgentBox |
| `airgap_scan_injection` | Classify untrusted text for instruction overrides | GMI MaaS |
| `airgap_redact_secrets` | Scrub recognizable credentials and high-entropy tokens | Local process |
| `airgap_audit_supply` | Inspect npm, requirements.txt, and Cargo manifests | Local process |
| `airgap_configure` | Change the model and timeout defaults | Local process |
| `airgap_audit_repo` | Run a six-phase defensive source review | AgentBox + MaaS |

No local Docker or Python runtime is required. Cloud tools are billable; local scanners and tool discovery work without credentials.

## Quick start

Install [Rust 1.89 or newer](https://www.rust-lang.org/tools/install) and your platform's native linker, then build from source:

```sh
git clone https://github.com/dhruvil-codes/airgap.git
cd airgap
cargo install --path . --locked
```

Set `GMI_AGENTBOX_API_KEY` to a compute-scoped key and `GMI_MAAS_API_KEY` to an inference key in the environment that starts your MCP client. See [.env.example](.env.example) for supported configuration names. Keep real credentials out of version control.

Add this server entry to your MCP client's configuration:

```json
{
  "mcpServers": {
    "airgap": {
      "command": "airgap",
      "env": {
        "GMI_AGENTBOX_API_KEY": "your-compute-key",
        "GMI_MAAS_API_KEY": "your-inference-key"
      }
    }
  }
}
```

The values above are placeholders. Use an absolute binary path if your client cannot find `airgap` on PATH. Cursor uses `.cursor/mcp.json`; Claude Desktop uses `claude_desktop_config.json`; Claude Code supports the `mcpServers` block in its user MCP configuration. These client-specific integrations have not been tested live.

`airgap` is a stdio server, so running it directly waits for an MCP client. Stdout carries JSON-RPC only; diagnostics go to stderr.

For a first tool call, pass the arguments in [examples/sandbox-run.json](examples/sandbox-run.json) to `airgap_sandbox_run`. It uploads a small Rust project and runs its test suite remotely. There is also a [local redaction example](examples/redact-secrets.json).

## Security boundary

GMI sandboxes have outbound internet. The name refers to the host execution boundary; this is not an offline or physically air-gapped system. Uploaded files leave your machine.

Only explicit snapshots are uploaded. The server does not read your working tree automatically. Adding this MCP server does not disable the agent's other terminal tools; configure your client's permissions separately.

Scanners and model reports can be wrong. Repository findings are static-review judgments, and SHA-256 receipts are unsigned integrity records. Provider isolation and remote network policy remain provider responsibilities.

Read the [threat model](docs/security-model.md) and [security reporting policy](SECURITY.md).

## Documentation

- [Tool inputs, configuration, limits, and cleanup](docs/reference.md)
- [Architecture and source map](docs/architecture.md)
- [Validation results and known limitations](docs/validation.md)
- [Contributing](CONTRIBUTING.md) and [community expectations](CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md) and [release process](docs/releasing.md)

## Development

```sh
cargo fmt --check
cargo check --locked
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

The automated tests run offline with local mocks and a real stdio child process. No GMI credentials are needed. See [CONTRIBUTING.md](CONTRIBUTING.md) for the Windows memory workaround and guidance on live tests.

## License

Available under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Maintained by [Dhruvil Mistry](https://github.com/dhruvil-codes). This is an independent project and is not an official GMI Cloud product.
