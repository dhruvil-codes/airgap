# Security model

## Intended boundary

airgap separates caller-submitted command execution from the machine hosting the MCP server. Commands and explicit file snapshots go to GMI AgentBox. The server has no API that executes a submitted command on the local host or automatically reads a local repository.

GMI sandboxes have outbound internet. This project does not implement an offline environment, certify microVM isolation, prove a CVE mitigation, or enforce the AI client's other tool permissions.

## What leaves the machine

| Operation | Data sent to the provider |
| --- | --- |
| Sandbox execution | Explicit file contents, selected image and command |
| Patch verification | Base snapshot, diff and test command |
| Injection scan | Submitted text and optional source label |
| Repository audit | Heuristically redacted source, fixed inventory output and prior review data |
| Local secret and supply scanners | Nothing |

Sandbox and verification snapshots are uploaded unchanged. Do not submit secrets unless you intend the cloud environment to receive them. Key headers stay in the provider client; host credentials are not forwarded as sandbox environment variables.

## Host-side controls

The explicit `setup`/`init` commands write local editor configuration and routing instructions, separate from MCP mode. They can store discovered keys in workspace and user config files. See [setup credential handling](setup.md#credentials). Symlinked config files and immediate parent directories are refused rather than redirecting credential writes.

- Snapshot paths must be relative, with no traversal, Windows drive prefixes, control characters or `.git` metadata. Files are regular UTF-8 entries with mode 0644.
- Snapshot size, file count, command length, scanner input and provider response size are bounded.
- Provider configuration requires HTTPS. Redirects and automatic proxies are disabled.
- Cloud requests and executions have deadlines. Sandbox lifetimes are explicit and cleanup is awaited.
- Provider errors omit raw response bodies. Returned logs receive heuristic redaction and exact replacement of configured provider keys.
- Contradictory classifier output, missing exit codes and invalid model JSON cannot become successful verification.

These controls are enforced by this client. Image trust, sandbox isolation, network egress, provider retention and account authorization remain separate responsibilities.

## Residual risks

Environment-file discovery is a trusted configuration mechanism. Start the server from a directory you trust; discovered files can select credentials, models and provider destinations.

Forced termination, an outage or a lost create response can prevent cleanup. Sandbox expiry bounds sandbox lifetime, but abandoned templates may need console cleanup. A client cancellation alone does not cancel the cleanup worker.

Secret scanners can miss credentials and redact harmless text. Manifest checks are heuristics without package-source, transitive-dependency or advisory analysis. Model classifiers and audits can produce false positives or false negatives. Never use those outputs as authorization to disclose data or perform sensitive actions.

Hostile tests can forge stdout, summaries and exit codes. A successful test command does not prove that every intended test ran. Receipts detect changes to the recorded data; an unsigned receipt cannot prove who produced it or that remote execution occurred.

## Disclosure

Follow [SECURITY.md](../SECURITY.md) for reports about airgap. The [validation record](validation.md) describes observed behavior and remaining gaps.
