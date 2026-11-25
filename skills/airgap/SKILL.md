---
name: airgap
description: Isolate untrusted code execution, verify GitHub pull requests, scan for prompt injection, audit dependency manifests for supply-chain risks, and redact secrets from logs using the airgap MCP server.
---

# Airgap Security Boundary

Use the `airgap` MCP tools whenever an agent needs to inspect or execute untrusted code, PRs, patches, or dependencies without exposing the host workstation.

## Tool Routing

- **`airgap_verify_pr`**: Verify a GitHub PR (`owner/repo#number` or URL). Scans title/body/diff for prompt injection, audits changed dependency manifests, clones at the pinned head SHA inside a disposable Linux microVM, runs build/tests, destroys the VM, and returns a `VERIFIED` or `FAILED` verdict with secrets redacted.
- **`airgap_verify_patch`**: Apply a local unified diff to clean base files inside an ephemeral microVM and run tests without touching the host working tree.
- **`airgap_sandbox_run`**: Run ad-hoc commands, tests, builds, or repro scripts inside a fresh microVM (`rust:1-bookworm`, `node:20-bookworm`, or `python:3.12-bookworm`).
- **`airgap_scan_injection`**: Screen untrusted issue text, PR descriptions, or scraped content for prompt-injection payloads before acting on them.
- **`airgap_audit_supply`**: Audit `package.json`, `Cargo.toml`, or `requirements.txt` locally (zero keys needed) for `preinstall`/`postinstall` hooks, git URLs, and risky dependencies.
- **`airgap_redact_secrets`**: Scrub API keys, bearer tokens, private key blocks, and high-entropy secrets locally with `[REDACTED]`.
- **`airgap_audit_repo`**: Run a 6-phase defensive static security review of a repository.
