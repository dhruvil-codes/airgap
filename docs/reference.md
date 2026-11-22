# Tool and configuration reference

Start with the [project README](../README.md) for installation and an overview.

For automatic registration, use `airgap setup --yes`. The [setup guide](setup.md) covers CLI dispatch, editor detection, credential storage, routing rules and doctor. The manual JSON examples below remain available as an alternative.

A Rust MCP server that sends explicitly supplied files and commands to disposable GMI AgentBox sandboxes. It also provides local secret redaction, manifest inspection, and model-assisted defensive review through GMI MaaS. It never executes submitted commands on your computer.

The name describes the intended host boundary. GMI sandboxes have outbound internet, so this is not a physically air-gapped or offline system. Files sent to a sandbox or inference model leave your machine. Provider isolation, network policy, account permissions, and image trust remain part of the security boundary.

## Build and run

Requires Rust 1.89 or newer and a native linker. The server needs no local Docker or Python installation.

```sh
cargo build --release --locked
cargo install --path . --locked
airgap
```

`airgap` speaks newline-delimited MCP JSON-RPC on stdin/stdout. It waits for an MCP client, rather than displaying a terminal interface. Diagnostics go to stderr. On Windows the binary is `target/release/airgap.exe`.

Set credentials in the environment that starts the MCP client. AgentBox usually needs a compute-scoped key; an inference-only key may not authorize sandbox operations.

```sh
export GMI_AGENTBOX_API_KEY="your-compute-key"
export GMI_MAAS_API_KEY="your-inference-key"
```

PowerShell:

```powershell
$env:GMI_AGENTBOX_API_KEY = "your-compute-key"
$env:GMI_MAAS_API_KEY = "your-inference-key"
```

Local scanners and tool discovery work without credentials. Cloud tools return a tool error when credentials are absent. No cloud calls occur on startup.

## MCP client setup

For Cursor, put this in `.cursor/mcp.json`. For Claude Desktop, use its `claude_desktop_config.json`. The `env` values below are placeholders; keep real keys out of version control.

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

For Claude Code, the same `mcpServers` block can be added to the appropriate user MCP configuration in `~/.claude.json`. The client must be able to find `airgap` on PATH; otherwise use an absolute binary path. Restart the client after changing its configuration.

Adding an MCP server does not disable an agent's other terminal tools. Configure the client's tool permissions separately if all execution must pass through airgap.

## Tools

| Tool | Inputs | Behavior |
| --- | --- | --- |
| `airgap_sandbox_run` | `command`; optional `files`, `image`, `timeout_secs` | Creates a template and sandbox, uploads an in-memory tar.gz, executes remotely, polls, redacts output, then deletes both resources. Returns exit code, stdout, stderr, duration and passed status. |
| `airgap_verify_patch` | `diff`, `test_command`, **`files`**; optional `image`, `timeout_secs` | Applies a diff to the supplied base snapshot using `git apply --check` and `git apply`, then runs tests. Returns execution evidence and an unsigned SHA-256 receipt. |
| `airgap_scan_injection` | `text_content`; optional `source_label` | MaaS classification into clean, suspicious, or malicious. Invalid, truncated or refused output is an error, never a clean verdict. |
| `airgap_redact_secrets` | `raw_text` | Local regex and entropy scrubbing with a merged-match count. |
| `airgap_audit_supply` | `manifest_type`: npm/python/cargo; `manifest_content` | Local review of lifecycle hooks, explicit build scripts and alternate dependency sources. |
| `airgap_configure` | optional `model`, `timeout_seconds` | Atomically changes defaults for subsequent requests. Does not change provider URLs or credentials. |
| `airgap_audit_repo` | `files`; optional `image` | Fixed remote inventory followed by six defensive static-review phases. Returns report Markdown, source-reviewed findings, rejected hypotheses and a receipt. |

Example `airgap_sandbox_run` arguments:

```json
{
  "command": "cargo test --quiet",
  "files": {
    "Cargo.toml": "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    "src/lib.rs": "#[test]\nfn arithmetic() { assert_eq!(2 + 2, 4); }\n"
  },
  "timeout_secs": 120
}
```

The default image is `docker.io/library/rust:1-bookworm` for a root Cargo.toml, `docker.io/library/node:20-bookworm` for a root package.json, or `docker.io/library/python:3.12-bookworm` otherwise. Override it for other stacks. Images must provide `sh`, `tar`, `mkdir` and `sleep`; verification also needs `git`. Pin an image digest for repeatable runs. Files are UTF-8 regular files with mode 0644, without executable bits or symlinks. The remote working directory is `/workspace`. `/tmp` is a 1 GiB tmpfs, so commands run with `TMPDIR=/root/tmp` on the root disk. Setup runs `corepack enable` when the image has Corepack, so `pnpm` and `yarn` work without a global npm install, and `PNPM_HOME=/root/.local/share/pnpm` is on `PATH`.

A sandbox run is `passed` only when the exit code is 0, the output contains no `Killed` or `SIGKILL` marker, and any recognized Rust or pytest summary reports zero failures. Exit code 137 is reported as an out-of-memory kill with a suggestion to use a larger instance type.

Patch verification requires a clean base snapshot because a diff alone cannot reconstruct a repository. It never reads the host working tree. `verified` requires command exit 0, untruncated logs, a recognized Rust or pytest summary, at least one passing test, and zero reported failures. Unknown summaries return null counts and `verified: false`, even if `command_passed` is true. Test summaries and exit codes are evidence supplied by the remote process; hostile tests can forge them. Exit zero cannot prove all intended tests ran. Zero-test runs are never verified.

Repository audit uses six phases: recon, control inventory, candidate identification, skeptical verification, consolidation, and reporting. It never runs generated exploits, generates attack scripts, or probes external targets. The compatibility field `confirmed_vulnerabilities` contains model source-review judgments whose exact file/line quotes matched the snapshot. It does **not** establish runtime exploitability. Unmatched quotes become rejected hypotheses. This is a defensive workflow inspired by staged reviews; it is not a certified implementation of a Cloudflare methodology. Reports are returned through MCP; the server does not write a local REPORT.md.

## Configuration

The loader searches the current directory and four parents. In each directory it reads `.env` before `cloudvet.env`. The closest file wins, with the process environment taking precedence over all files. It parses values without executing shell commands or changing the process environment. `GMI_AGENTBOX_BASE_URL` and `GMI_MAAS_BASE_URL` are read only from the process environment; values in environment files are ignored so a cloned repository cannot redirect credentials. Start from a trusted directory because environment files still control credentials and models.

| Variable | Default / meaning |
| --- | --- |
| `GMI_API_KEY` | Fallback when a dedicated key is absent |
| `GMI_AGENTBOX_API_KEY` | Compute-scoped AgentBox key |
| `GMI_MAAS_API_KEY` | Inference key |
| `GMI_DEFAULT_IDC` | `us-central-iowa16` |
| `GMI_AGENTBOX_INSTANCE_TYPE` | `gmi.sandbox.medium` (2 vCPU, 4 GiB). `gmi.sandbox.small` is 1 vCPU, 2 GiB at about half the price; `gmi.sandbox.large` is 4 vCPU, 8 GiB |
| `GMI_MODEL` | `deepseek-ai/DeepSeek-V4.1-Flash` |
| `GMI_AGENTBOX_BASE_URL` | `https://api.gmi-serving.com/v1/agents` |
| `GMI_MAAS_BASE_URL` | `https://api.gmi-serving.com/v1` |

Model identifiers and regions must be available to your account. The requested default model is configurable; availability is not assumed or checked at startup. The MaaS base includes `/v1`, and the client appends `/chat/completions` once.

Provider overrides require HTTPS without URL credentials, queries, or fragments. Redirects and automatic proxies are disabled. Each HTTP operation has a bounded timeout and a 3 MiB response cap. Runtime execution defaults to 120 seconds, accepts 1..1800 seconds, and allows a further 600 seconds for provisioning. A fresh template build can take minutes. A sandbox gets an explicit lifetime of the command budget plus 180 seconds, which also covers readiness and extraction. Readiness taking longer can consume that lifetime.

At most four sandbox jobs and four inference requests run concurrently. Saturated requests fail with a retry message. Snapshots allow 2000 files and 8 MiB total path/content bytes. Paths must be relative, without traversal, Windows drive prefixes, control characters, or `.git` metadata. Text scanner input is capped at 2 MiB. The serialized inference envelope must also fit 2 MiB; large audit snapshots may need to be reduced to relevant files. The server never uploads files implicitly.

## Cleanup and evidence

Cleanup is awaited on success, failure, timeout and graceful shutdown. A cancelled MCP caller cannot drop its cloud cleanup task. Deletion removes the sandbox before its template, attempts both even if one fails, and retries transient failures and template-in-use conflicts. If a run succeeded but cleanup did not, the result is still returned with a `cleanup_warning`; if the run failed too, the error notes the cleanup failure. Graceful shutdown cancels active jobs and waits up to 35 seconds for cleanup. Forced termination, a lost create response or a provider outage can leave a resource behind. Sandbox expiry bounds sandbox lifetime, but abandoned templates may still require console cleanup.

Receipts hash the serialized input and output with SHA-256, plus metadata and optional provider-reported token usage. To verify the outer hash, deserialize as `Receipt`, set `sha256` to an empty string, and hash the struct's JSON serialization in declared field order. `Receipt::verify` implements this. These are unsigned integrity records, not provider-signed attestations or proof of execution. Sandbox execution has no model token usage, so its receipt usage is null. Patch input hashing includes the full request and base snapshot.

Secret scrubbing detects credential assignments, AWS access IDs, common GitHub/OpenAI/Slack tokens, JWTs, private keys and high-entropy strings. It can miss secrets and can redact harmless strings. It does not promise vectorization. Explicit files for sandbox execution and verification are uploaded unchanged. Audit source is heuristically scrubbed before upload and inference. Prompt classification is fallible and must never authorize sensitive actions by itself. Manifest checks do not query an advisory database, download dependencies, inspect package code, or resolve transitive dependencies.

## Verification

```sh
cargo fmt --check
cargo check --locked
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

Tests use a real child process for MCP initialize/list/call, local HTTP mocks for lifecycle, failure, cancellation, timeouts and inference parsing, and unit tests for path validation, archives, configuration, receipts and scanners. They need no GMI credentials and create no billable resources. Mock coverage does not prove live provider compatibility. Separate credentialed smoke checks verified live sandbox execution, file upload, redaction, injection classification and the six-phase audit. Live patch verification was rejected with HTTP 403 at the AgentBox execution endpoint; it is not claimed to work end to end on the tested account. See [the validation record](validation.md).

Release builds use LTO, one codegen unit, stripped symbols and abort-on-panic. A Linux static build requires a musl target and its linker:

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --release --locked --target x86_64-unknown-linux-musl
```

The <3 ms startup, <10 MB resident memory, <8 MB binary and sub-millisecond protocol latency figures in the design brief are targets, not established guarantees. See [the validation record](validation.md) for measurements on this machine. Native Windows builds are not Linux static artifacts.

## Provider contracts

The AgentBox client follows the official [GMI Python SDK 0.1.0b8 request shapes](https://pypi.org/project/gmi-agentbox-sdk/) using the gateway at `/v1/agents`: create/read/delete `/deployments`, launch `/deployments/{slug}/tasks`, poll/delete `/tasks/{id}`, upload multipart `/tasks/{id}/files?path=...`, and run/poll/cancel `/tasks/{id}/executions`. No Python code is used at runtime.

GMI also documents a [console v2 control API and direct sandbox data plane](https://www.gmicloud.ai/en/blog/agentbox-v2-one-agent-ran-47-trending-github-repos-in-47-sandboxes). Its raw paths and credentials differ from the SDK gateway. The client does not guess interchangeable endpoints. Inference follows the [official OpenAI-compatible API](https://www.gmicloud.ai/en/developers).

No specific CVE mitigation claim is made. Host loopback is not mounted or forwarded by this server; remote network reachability and microVM isolation are provider responsibilities.

## License

MIT OR Apache-2.0.
