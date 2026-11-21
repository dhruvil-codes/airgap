# airgap

[![CI](https://github.com/dhruvil-codes/airgap/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/dhruvil-codes/airgap/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.89%2B-orange?logo=rust)](https://www.rust-lang.org/tools/install)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![M8ven Trust Score](https://m8ven.ai/api/mcp/badge?repo=dhruvil-codes/airgap)](https://m8ven.ai/mcp/dhruvil-codes/airgap)

**Remote execution and defensive security tools for AI coding agents.**

airgap is a Rust [Model Context Protocol](https://modelcontextprotocol.io/) server. It sends explicitly supplied files and commands to disposable GMI AgentBox sandboxes and exposes local scanners and model-assisted security review over stdio. Submitted commands never execute on the machine running the server.

> **Early development.** Offline tests and release builds pass on Linux, macOS and Windows. Live sandbox execution and inference work with the tested account from Windows. Patch verification currently receives HTTP 403 from GMI's execution endpoint. See the [validation record](docs/validation.md) before relying on the results.

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

### Install and set up in one command

macOS or Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts/install.ps1 | iex
```

Installers fetch a published release, verify its SHA-256 checksum, install the binary, and run `airgap setup --yes`. No Rust or Python is required. If no release has been published yet, use Cargo below. Download time and client approvals mean five-second setup is a goal, not a guarantee.

### Or install from source with Cargo

Requires [Rust 1.89 or newer](https://www.rust-lang.org/tools/install) and a native linker:

```sh
cargo install --git https://github.com/dhruvil-codes/airgap --locked
airgap setup
```

This uses the repository directly; a crates.io publication is not assumed. Run setup from your workspace. It discovers GMI keys in the environment or trusted `.env` / `cloudvet.env` files. Missing keys do not block configuration: local scanners work, and you can set `GMI_AGENTBOX_API_KEY` and `GMI_MAAS_API_KEY` and rerun setup later. See [.env.example](.env.example).

```sh
airgap setup --yes              # detected editors; all three if none detected
airgap init --cursor --yes      # Cursor configuration only
airgap setup --rules-only --yes # workspace routing instructions only
airgap doctor                  # bounded credential/connectivity checks
```

Setup merges Cursor, Claude Code and VS Code settings without removing other servers, uses the executable's absolute path, and preserves team instructions around its managed rule block. Repeated runs leave identical files unchanged. Discovered keys are stored in editor config files; keep those files out of version control.

Reload your client, approve the server if requested, and prompt normally, for example `run tests`. Routing rules guide the agent toward remote tools; they cannot enforce tool permissions or guarantee tool selection. See [the setup guide](docs/setup.md) for exact destinations and flags.

No arguments or `--mcp` serves MCP JSON-RPC on stdout. Help, version, setup and doctor are terminal commands and do not start an MCP session.

For a first sandbox call, use [examples/sandbox-run.json](examples/sandbox-run.json). There is also a [local redaction example](examples/redact-secrets.json).

## Security boundary

GMI sandboxes have outbound internet. The name refers to the host execution boundary; this is not an offline or physically air-gapped system. Uploaded files leave your machine.

Only explicit snapshots are uploaded. The server does not read your working tree automatically. Adding this MCP server does not disable the agent's other terminal tools; configure your client's permissions separately.

Scanners and model reports can be wrong. Repository findings are static-review judgments, and SHA-256 receipts are unsigned integrity records. Provider isolation and remote network policy remain provider responsibilities.

Read the [threat model](docs/security-model.md) and [security reporting policy](SECURITY.md).

## Documentation

- [Automatic setup, installer options, and doctor](docs/setup.md)
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
