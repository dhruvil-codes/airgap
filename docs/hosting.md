# Public Hosting & Remote Deployment

Airgap supports both local stdio (`airgap --mcp`) and native remote Streamable HTTP / SSE (`airgap serve`). You can host Airgap publicly so teams and AI agents connect over HTTPS without installing Rust or local binaries.

## Recommended Strategy

1. **Fastest public adoption (Day 1): Glama.ai** (or **NodeFlare**) using the root `Dockerfile` (`CMD ["--mcp"]`).
   - Zero infrastructure to maintain: Glama/NodeFlare builds the container directly from GitHub, handles OAuth 2.1 / API key issuance, manages per-user container isolation, and gives Airgap instant discovery in the MCP directory.
   - Ideal for solo developers trying Airgap immediately with their own GMI keys supplied via platform secrets.
2. **Production custom domain & multi-tenant control: Fly.io (`airgap serve`)** using `fly.toml` and the same `Dockerfile`.
   - Scales to zero (`min_machines_running = 0`, wakes in <300ms), supports a custom domain (`https://mcp.airgap.dev/mcp`), enforces native constant-time Bearer auth, per-tenant concurrency slots, and BYOK headers (`X-GMI-AgentBox-Key`, `X-GMI-MaaS-Key`) so external users never burn the host operator's GMI compute quota.

---

## Path A: Turnkey Registry Hosting (Glama / NodeFlare)

The root `Dockerfile` builds a stripped Rust 1.89 release binary into a minimal `debian:bookworm-slim` image running as a non-root `airgap` user with `ENTRYPOINT ["airgap"]` and `CMD ["--mcp"]`.

1. Connect `https://github.com/dhruvil-codes/airgap` in [Glama](https://glama.ai/mcp) or [NodeFlare](https://nodeflare.tech).
2. Configure environment variables (`GMI_AGENTBOX_API_KEY`, `GMI_MAAS_API_KEY`, optional `GITHUB_TOKEN`) in the platform's per-user secret prompts.
3. The platform exposes a managed Streamable HTTP endpoint (for example `https://airgap.glama.ai/mcp`).

---

## Path B: Self-Hosted Streamable HTTP on Fly.io or Railway

### Running `airgap serve`

```sh
# Local development (loopback)
airgap serve --port 8080

# Production (non-loopback requires bearer auth by default)
export AIRGAP_AUTH_KEY="replace-with-strong-random-token"
export AIRGAP_ALLOWED_HOSTS="mcp.airgap.dev,airgap-mcp.fly.dev"
airgap serve --host 0.0.0.0 --port 8080 --require-byok
```

Endpoints:
- `POST /mcp`, `GET /mcp`, `DELETE /mcp`: MCP Streamable HTTP / SSE transport (16 MiB body limit).
- `GET /healthz`: unauthenticated JSON health check (`{"status":"ok","service":"airgap","version":"0.2.0"}`).

### Deploying to Fly.io

The repository ships a ready-to-use `fly.toml` with scale-to-zero and `/healthz` checks:

```sh
fly launch --no-deploy --copy-config
fly secrets set \
  AIRGAP_AUTH_KEY="$(openssl rand -hex 32)" \
  AIRGAP_REQUIRE_BYOK="1" \
  AIRGAP_ALLOWED_HOSTS="airgap-mcp.fly.dev,mcp.airgap.dev"
fly deploy
```

> **Managed pool vs BYOK**: Set `AIRGAP_REQUIRE_BYOK=1` (recommended for public endpoints) so callers must provide their own GMI keys via HTTP headers and the server never falls back to host keys. For private team deployments sharing one GMI account, omit `AIRGAP_REQUIRE_BYOK` and set `GMI_AGENTBOX_API_KEY`, `GMI_MAAS_API_KEY`, and `AIRGAP_AUTH_KEYS="team-key-1,team-key-2"` (one bearer token per user/team for isolated concurrency slots).

### Deploying to Railway

1. Create a new Railway service linked to the GitHub repository (auto-detects `Dockerfile`).
2. Set the container Start Command to:
   ```sh
   serve --host 0.0.0.0 --port $PORT
   ```
3. Set environment variables: `AIRGAP_AUTH_KEY` (or `AIRGAP_AUTH_KEYS`), `AIRGAP_REQUIRE_BYOK=1`, and optional `AIRGAP_ALLOWED_HOSTS`. (`RAILWAY_PUBLIC_DOMAIN` is automatically added to allowed `Host` headers.)
4. Set Healthcheck Path to `/healthz`.

---

## Multi-Tenant Security Controls

| Control | How `airgap serve` enforces it |
| :--- | :--- |
| **Fail-closed auth** | Binding to a non-loopback address (`0.0.0.0`) refuses to start unless `AIRGAP_AUTH_KEY` / `AIRGAP_AUTH_KEYS` / `--auth-key` is configured (or `--allow-unauthenticated` is explicitly passed). Bearer tokens are compared in constant time via SHA-256 digests. |
| **Credential segregation (BYOK)** | Callers can pass `X-GMI-AgentBox-Key`, `X-GMI-MaaS-Key`, `X-GMI-API-Key`, and `X-GitHub-Token` per request. With `--require-byok` (`AIRGAP_REQUIRE_BYOK=1`), server credentials are never used as a fallback. BYOK keys are scoped strictly to the request, scrubbed from sandbox output, and never logged or written to disk. |
| **Per-tenant slot & config isolation** | Concurrency semaphores (`--max-slots-per-tenant`, default `4` each for AgentBox and MaaS) and `airgap_configure` state are isolated per tenant ID (derived from authenticated bearer token and/or BYOK key hash). One tenant saturating its slots or changing its model never affects other tenants. |
| **Template lock isolation** | BYOK tenants get independent GMI template build locks keyed by their compute key hash so cold builds in one GMI account never stall another account. |
| **Host & CORS / Origin protection** | Inbound `Host` headers are validated against an allowlist (preventing DNS rebinding). Browser `Origin` headers are rejected by default (`403 Forbidden`) unless explicitly listed via `--allowed-origins` / `AIRGAP_ALLOWED_ORIGINS`. Non-browser MCP clients without an `Origin` header pass cleanly. |

---

## Client Configuration Guide

Replace `https://mcp.airgap.dev/mcp` with your deployed URL (e.g. `https://airgap-mcp.fly.dev/mcp` or `https://airgap.glama.ai/mcp`). Omit the `X-GMI-*` headers if connecting to a managed-pool server.

### Claude Desktop (`claude_desktop_config.json`)

Claude Desktop connects to remote Streamable HTTP MCP servers via **Settings -> Connectors -> Add custom connector** (using your HTTPS URL), or in `claude_desktop_config.json` via `mcp-remote`:

```json
{
  "mcpServers": {
    "airgap": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "https://mcp.airgap.dev/mcp",
        "--header",
        "Authorization: Bearer ${AIRGAP_AUTH_KEY}",
        "--header",
        "X-GMI-AgentBox-Key: ${GMI_AGENTBOX_API_KEY}",
        "--header",
        "X-GMI-MaaS-Key: ${GMI_MAAS_API_KEY}"
      ]
    }
  }
}
```

### Claude Code (`.mcp.json` or `~/.claude.json`)

```json
{
  "mcpServers": {
    "airgap": {
      "type": "http",
      "url": "https://mcp.airgap.dev/mcp",
      "headers": {
        "Authorization": "Bearer ${AIRGAP_AUTH_KEY}",
        "X-GMI-AgentBox-Key": "${GMI_AGENTBOX_API_KEY}",
        "X-GMI-MaaS-Key": "${GMI_MAAS_API_KEY}"
      }
    }
  }
}
```

Or via CLI:

```sh
claude mcp add --transport http airgap https://mcp.airgap.dev/mcp \
  --header "Authorization: Bearer $AIRGAP_AUTH_KEY" \
  --header "X-GMI-AgentBox-Key: $GMI_AGENTBOX_API_KEY" \
  --header "X-GMI-MaaS-Key: $GMI_MAAS_API_KEY"
```

### Cursor (`.cursor/mcp.json` or `~/.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "airgap": {
      "url": "https://mcp.airgap.dev/mcp",
      "headers": {
        "Authorization": "Bearer ${env:AIRGAP_AUTH_KEY}",
        "X-GMI-AgentBox-Key": "${env:GMI_AGENTBOX_API_KEY}",
        "X-GMI-MaaS-Key": "${env:GMI_MAAS_API_KEY}"
      }
    }
  }
}
```

### Factory Droid (`~/.factory/mcp.json` or `.factory/mcp.json`)

```json
{
  "mcpServers": {
    "airgap": {
      "type": "http",
      "url": "https://mcp.airgap.dev/mcp",
      "headers": {
        "Authorization": "Bearer ${AIRGAP_AUTH_KEY}",
        "X-GMI-AgentBox-Key": "${GMI_AGENTBOX_API_KEY}",
        "X-GMI-MaaS-Key": "${GMI_MAAS_API_KEY}"
      }
    }
  }
}
```

> Keep API keys out of committed workspace files (`.mcp.json`, `.cursor/mcp.json`, `.factory/mcp.json`); prefer user-level configs or environment variable references.
