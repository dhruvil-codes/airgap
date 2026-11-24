---
title: airgap
description: Run your coding agent's installs, builds and tests in disposable Linux microVMs, not on your machine.
version: 0.2.0
license: MIT OR Apache-2.0
repository: https://github.com/dhruvil-codes/airgap
url: https://www.useairgap.dev/
---

# airgap

**Let your agent run anything. Just not on your machine.**

airgap is a Rust MCP server that moves your agent's installs, builds and tests into a fresh Linux microVM on GMI Cloud. It scans untrusted text first, audits manifests before install, and scrubs secrets from the logs your agent reads back.

Works with Claude Code, Cursor, VS Code, Factory Droid and any stdio MCP client.

## Install

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts/install.sh | bash
# Windows (PowerShell)
irm https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts/install.ps1 | iex
# From source (not published on crates.io)
cargo install --git https://github.com/dhruvil-codes/airgap --locked
```

Register it with your client and check the setup:

```sh
airgap setup --claude --yes     # or --cursor, --vscode, --droid
airgap doctor
```

`airgap setup` writes routing blocks into AGENTS.md, CLAUDE.md and .cursorrules (and .github/copilot-instructions.md for VS Code).

For other MCP clients, such as Windsurf:

```json
{ "mcpServers": { "airgap": { "command": "airgap", "args": ["--mcp"] } } }
```

Then ask in plain words: `use airgap to verify acme/api#142`, `use airgap and run the tests`, `audit this package.json with airgap`.

## The problem

- **Blast radius.** A postinstall script runs with your user's permissions and can read SSH keys, cloud credentials and every checked-out repository.
- **Wrong machine.** On Windows or macOS, Linux-only scripts, POSIX permissions and glibc builds fail locally.
- **Untrusted input.** PR descriptions, issues and manifests can carry instructions aimed at your agent.

## How one call works (airgap_verify_pr)

1. **Scan**: PR title and body checked for prompt injection. Malicious stops here, before any VM exists.
2. **Audit**: changed manifests checked for lifecycle hooks. Risky ones block by default (`fail_on_supply_risk = true`).
3. **Clone**: a fresh microVM clones the repo pinned to the PR's head SHA.
4. **Build**: Node, Python or Rust image picked from the files.
5. **Test**: changed files paired with sibling tests.
6. **Scrub**: tokens and keys replaced with `[REDACTED]`.
7. **Verdict**: VERIFIED or FAILED, with logs and an unsigned SHA-256 receipt.

## Security gates

| Gate | Tool | Runs on | What it does |
| --- | --- | --- | --- |
| Injection | `airgap_scan_injection` | GMI MaaS | Classifies text as clean, suspicious or malicious |
| Supply chain | `airgap_audit_supply` | local | Flags lifecycle scripts; `allow_install_scripts` defaults to false |
| Isolation | `airgap_sandbox_run` and others | microVM | Fresh gmi.sandbox.medium (2 vCPU, 4 GiB) per run, deleted afterwards |
| Secrets | `airgap_redact_secrets` | local | Replaces credentials with `[REDACTED]` |

## Tools

| Tool | Use it when | Runs on |
| --- | --- | --- |
| `airgap_verify_pr` | A GitHub PR needs verifying | GMI MaaS, local, microVM |
| `airgap_verify_patch` | A local diff needs testing against clean base files | microVM |
| `airgap_sandbox_run` | Ad-hoc run, build, test, reproduce or scrape | microVM |
| `airgap_scan_injection` | Before acting on untrusted issue or PR text | GMI MaaS |
| `airgap_audit_supply` | Before installing from a manifest you did not write | local |
| `airgap_redact_secrets` | Before showing logs, diffs or env vars | local |
| `airgap_audit_repo` | Defensive security review (six static phases) | microVM, GMI MaaS |
| `airgap_configure` | Change default model or request timeout | local |

## Compared with your shell and local Docker

| | Agent's shell | Local Docker | airgap |
| --- | --- | --- | --- |
| Where commands run | Your machine | A container on your machine | A remote, single-use microVM |
| Your files in reach | All of them | Whatever you mount | Only what you send |
| Same Linux from any OS | No | Needs Docker Desktop | Yes |
| Prompt-injection check | No | No | Before any compute |
| Manifest audit before install | No | No | Lifecycle hooks flagged |
| Secrets in logs | Printed as is | Printed as is | Scrubbed before the agent reads |
| Cleanup after a run | Manual | Manual or --rm | Automatic, even on timeout |
| Needs installed locally | Nothing | Docker engine | One binary and a GMI key |

## Limits and FAQ

- **Not physically air-gapped.** Sandboxes have outbound internet; files you send leave your machine.
- **What leaves your machine:** only the files, command and text passed in a tool call.
- **It does not disable your agent's own terminal.** Routing rules make the agent prefer airgap; restrict client tool permissions if all execution must go through it.
- **Requirements:** the binary and a GMI Cloud account (AgentBox key for sandboxes, inference key for scans and audits). Local scanners need no credentials.
- **Speed:** the first run per image builds a template; commands default to 120 s, up to 1800 s; up to 4 sandbox jobs in parallel.
- **Cost:** airgap is free; GMI Cloud bills sandbox time and inference.
- **VERIFIED is strong evidence, not proof.** Receipts are unsigned SHA-256 records; zero-test runs are never marked verified.
- **Not an official GMI Cloud product.** An independent project by [Dhruvil Mistry](https://github.com/dhruvil-codes).

## For AI agents

- Index for language models: https://www.useairgap.dev/llms.txt (everything in one file: https://www.useairgap.dev/llms-full.txt)
- Read-only REST API: https://www.useairgap.dev/api/v1 (OpenAPI: https://www.useairgap.dev/openapi.json)
- Ask a question: `GET https://www.useairgap.dev/ask?query=...` (NLWeb; `streaming=true` for SSE)
- Docs MCP server (Streamable HTTP, no auth): https://www.useairgap.dev/mcp
- A2A agent card: https://www.useairgap.dev/.well-known/agent-card.json

## Links

- Docs: https://www.useairgap.dev/docs.md
- Guides: https://www.useairgap.dev/guides.md
- Setup: https://www.useairgap.dev/docs/setup.md
- Reference: https://www.useairgap.dev/docs/reference.md
- Hosting: https://www.useairgap.dev/docs/hosting.md
- Changelog: https://www.useairgap.dev/CHANGELOG.md
- About: https://www.useairgap.dev/about.md
- Contact: https://www.useairgap.dev/contact.md
- Repository: https://github.com/dhruvil-codes/airgap
