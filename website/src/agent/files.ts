import { AGENT_ENDPOINTS, CLIENT_SETUP, FAQ, INSTALL_COMMANDS, LIMITS, PAGES, ROUTING, SITE, TOOLS } from "./knowledge";

const O = SITE.origin;
const MCP_URL = `${O}/mcp`;
const A2A_URL = `${O}/a2a`;
export const MCP_PROTOCOL_VERSION = "2025-11-25";

const DOCS_MCP_DESCRIPTION =
  "Read-only documentation server for airgap, the Rust MCP server that runs AI coding agent commands in disposable GMI Cloud microVMs. Search the docs, look up the 8 airgap tools, and get install and client setup instructions. It does not run code; install the airgap binary for sandboxes.";

export const DOCS_MCP_TOOLS = [
  {
    name: "ask",
    title: "Ask about airgap",
    description: "Answer a natural-language question about airgap from its website and docs (same results as the NLWeb /ask endpoint).",
    inputSchema: { type: "object", required: ["query"], properties: { query: { type: "string", description: "The question." } } },
  },
  {
    name: "search_docs",
    title: "Search airgap docs",
    description: "Full-text search over every airgap page and doc. Returns sections with titles, URLs and text.",
    inputSchema: {
      type: "object",
      required: ["query"],
      properties: { query: { type: "string" }, limit: { type: "integer", minimum: 1, maximum: 20, default: 5 } },
    },
  },
  {
    name: "list_airgap_tools",
    title: "List airgap tools",
    description: "List the 8 tools of the airgap MCP server with when to use each one, what it needs and its input schema, plus the routing rules agents should follow.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "get_install_instructions",
    title: "Get install instructions",
    description: "Return the install command for a platform and the follow-up setup steps.",
    inputSchema: { type: "object", properties: { platform: { type: "string", enum: INSTALL_COMMANDS.map((i) => i.platform) } } },
  },
  {
    name: "get_client_setup",
    title: "Get MCP client setup",
    description: "Return the airgap setup command or JSON config for an MCP client.",
    inputSchema: { type: "object", properties: { client: { type: "string", enum: CLIENT_SETUP.map((c) => c.client) } } },
  },
].map((t) => ({ ...t, annotations: { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false } }));

export function knowledgeJson() {
  return {
    site: SITE,
    tools: TOOLS,
    routing: ROUTING,
    install: INSTALL_COMMANDS,
    clientSetup: CLIENT_SETUP,
    limits: LIMITS,
    faq: FAQ,
    pages: PAGES.map((p) => ({ ...p, url: `${O}${p.path === "/" ? "/" : p.path}`, markdownUrl: `${O}${p.markdown}` })),
    endpoints: AGENT_ENDPOINTS.map((e) => ({ ...e, url: `${O}${e.path}` })),
    docsMcpTools: DOCS_MCP_TOOLS,
  };
}

const err = { $ref: "#/components/responses/Error" };
const json = (schema: unknown, description = "OK") => ({ description, content: { "application/json": { schema } } });
const ref = (name: string) => ({ $ref: `#/components/schemas/${name}` });

export function openApi() {
  return {
    openapi: "3.1.0",
    info: {
      title: "airgap website API",
      version: SITE.version,
      summary: "Read-only API for facts about airgap: its MCP tools, docs search, install and setup.",
      description:
        "Public, unauthenticated, read-only API served by useairgap.dev. It answers questions about airgap and never runs code. To run commands in disposable microVMs, install the airgap binary and use its MCP tools. Errors are JSON objects of the form {\"error\":{\"code\",\"message\",\"docs\"}}.",
      contact: { name: SITE.author.name, url: `${O}/contact`, email: SITE.contactEmail },
      license: { name: "MIT OR Apache-2.0", identifier: "MIT OR Apache-2.0" },
    },
    externalDocs: { description: "llms.txt", url: `${O}/llms.txt` },
    servers: [{ url: O }],
    tags: [
      { name: "tools", description: "The 8 tools of the airgap MCP server" },
      { name: "docs", description: "Search and question answering over the docs" },
      { name: "setup", description: "Install and client setup" },
      { name: "meta", description: "Service status" },
    ],
    paths: {
      "/api/v1": { get: { operationId: "getApiIndex", tags: ["meta"], summary: "List API endpoints", responses: { "200": json(ref("ApiIndex")) } } },
      "/api/v1/health": { get: { operationId: "getHealth", tags: ["meta"], summary: "Service health", responses: { "200": json(ref("Health")) } } },
      "/api/v1/tools": {
        get: {
          operationId: "listAirgapTools",
          tags: ["tools"],
          summary: "List airgap MCP tools",
          description: "Returns every tool with when to use it, where it runs, the credentials it needs and its JSON input schema, plus agent routing rules.",
          responses: { "200": json({ type: "object", required: ["tools"], properties: { tools: { type: "array", items: ref("Tool") }, routing: { type: "array", items: ref("Route") } } }) },
        },
      },
      "/api/v1/tools/{name}": {
        get: {
          operationId: "getAirgapTool",
          tags: ["tools"],
          summary: "Get one airgap MCP tool",
          parameters: [{ name: "name", in: "path", required: true, schema: { type: "string", enum: TOOLS.map((t) => t.name) }, description: "Tool name" }],
          responses: { "200": json(ref("Tool")), "404": err },
        },
      },
      "/api/v1/search": {
        get: {
          operationId: "searchDocs",
          tags: ["docs"],
          summary: "Search the airgap docs",
          parameters: [
            { name: "q", in: "query", required: true, schema: { type: "string", minLength: 1, maxLength: 500 }, description: "Search text" },
            { name: "limit", in: "query", required: false, schema: { type: "integer", minimum: 1, maximum: 20, default: 5 } },
          ],
          responses: { "200": json(ref("SearchResponse")), "400": err },
        },
      },
      "/api/v1/install": {
        get: {
          operationId: "getInstallCommands",
          tags: ["setup"],
          summary: "Get install commands",
          parameters: [{ name: "platform", in: "query", required: false, schema: { type: "string", enum: INSTALL_COMMANDS.map((i) => i.platform) } }],
          responses: { "200": json({ type: "object", properties: { install: { type: "array", items: ref("InstallCommand") }, next: { type: "array", items: { type: "string" } } } }), "404": err },
        },
      },
      "/api/v1/setup": {
        get: {
          operationId: "getClientSetup",
          tags: ["setup"],
          summary: "Get MCP client setup",
          parameters: [{ name: "client", in: "query", required: false, schema: { type: "string", enum: CLIENT_SETUP.map((c) => c.client) } }],
          responses: { "200": json({ type: "object", properties: { setup: { type: "array", items: { type: "object" } } } }), "404": err },
        },
      },
      "/ask": {
        get: {
          operationId: "askGet",
          tags: ["docs"],
          summary: "Ask a question (NLWeb)",
          description: "NLWeb-compatible ask endpoint. Returns JSON by default; send streaming=true or Accept: text/event-stream for Server-Sent Events.",
          parameters: [
            { name: "query", in: "query", required: true, schema: { type: "string", minLength: 1, maxLength: 500 } },
            { name: "streaming", in: "query", required: false, schema: { type: "boolean", default: false } },
          ],
          responses: { "200": { description: "Answer", content: { "application/json": { schema: ref("AskResponse") }, "text/event-stream": { schema: { type: "string" } } } }, "400": err },
        },
        post: {
          operationId: "askPost",
          tags: ["docs"],
          summary: "Ask a question (NLWeb, POST)",
          requestBody: {
            required: true,
            content: { "application/json": { schema: { type: "object", required: ["query"], properties: { query: { type: "string" }, streaming: { type: "boolean", default: false } } } } },
          },
          responses: { "200": { description: "Answer", content: { "application/json": { schema: ref("AskResponse") }, "text/event-stream": { schema: { type: "string" } } } }, "400": err },
        },
      },
    },
    components: {
      responses: { Error: json(ref("Error"), "Error") },
      schemas: {
        Error: {
          type: "object",
          required: ["error"],
          properties: { error: { type: "object", required: ["code", "message"], properties: { code: { type: "string" }, message: { type: "string" }, docs: { type: "string", format: "uri" } } } },
        },
        ApiIndex: { type: "object", properties: { name: { type: "string" }, version: { type: "string" }, endpoints: { type: "array", items: { type: "object" } } } },
        Health: { type: "object", required: ["status"], properties: { status: { type: "string", enum: ["ok"] }, service: { type: "string" }, version: { type: "string" } } },
        Tool: {
          type: "object",
          required: ["name", "summary", "inputSchema"],
          properties: {
            name: { type: "string" },
            title: { type: "string" },
            summary: { type: "string" },
            useWhen: { type: "string" },
            runsOn: { type: "array", items: { type: "string" } },
            needsCredentials: { type: "string" },
            inputSchema: { type: "object" },
            example: { type: "object" },
          },
        },
        Route: { type: "object", properties: { when: { type: "string" }, tool: { type: "string" }, note: { type: "string" } } },
        SearchResult: {
          type: "object",
          required: ["url", "title", "text"],
          properties: { url: { type: "string", format: "uri" }, title: { type: "string" }, section: { type: "string" }, text: { type: "string" }, score: { type: "number" } },
        },
        SearchResponse: { type: "object", required: ["query", "results"], properties: { query: { type: "string" }, results: { type: "array", items: ref("SearchResult") } } },
        InstallCommand: { type: "object", properties: { platform: { type: "string" }, label: { type: "string" }, command: { type: "string" } } },
        AskResponse: {
          type: "object",
          required: ["query_id", "results"],
          properties: {
            _meta: { type: "object" },
            query_id: { type: "string" },
            query: { type: "string" },
            results: {
              type: "array",
              items: {
                type: "object",
                properties: { url: { type: "string" }, name: { type: "string" }, site: { type: "string" }, score: { type: "number" }, description: { type: "string" }, schema_object: { type: "object" } },
              },
            },
          },
        },
      },
    },
  };
}

// SEP-2127 style card (server.json schema), served at /.well-known/mcp.json.
export function mcpServerJsonCard() {
  return {
    $schema: "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json",
    name: "dev.useairgap/airgap-docs",
    title: "airgap docs",
    description: "Read-only airgap docs: search pages, list the 8 sandbox tools, get install and client setup.",
    version: SITE.version,
    websiteUrl: O,
    repository: { url: SITE.repo, source: "github" },
    remotes: [{ type: "streamable-http", url: MCP_URL }],
    _meta: {
      "io.modelcontextprotocol.registry/publisher-provided": {
        auth: "none",
        readOnly: true,
        about: DOCS_MCP_DESCRIPTION,
        sandboxServer: { registryName: SITE.registryName, install: INSTALL_COMMANDS[0].command, stdio: { command: "airgap", args: ["--mcp"] } },
        tools: DOCS_MCP_TOOLS.map((t) => t.name),
      },
    },
  };
}

// SEP-1649 server card, served at /.well-known/mcp/server-card.json.
export function mcpServerCard() {
  return {
    $schema: "https://modelcontextprotocol.io/schemas/server-card/v1.0",
    version: "1.0",
    protocolVersion: MCP_PROTOCOL_VERSION,
    serverInfo: { name: "airgap-docs", title: "airgap docs", version: SITE.version, description: DOCS_MCP_DESCRIPTION, homepage: O },
    transport: { type: "streamable-http", url: MCP_URL },
    authentication: { required: false, schemes: [] },
    capabilities: { tools: { listChanged: false }, resources: { listChanged: false, subscribe: false }, prompts: { listChanged: false } },
    tools: DOCS_MCP_TOOLS,
    resources: [
      { uri: `${O}/llms.txt`, name: "llms.txt", mimeType: "text/plain" },
      { uri: `${O}/llms-full.txt`, name: "llms-full.txt", mimeType: "text/plain" },
    ],
    instructions: "Use these tools to learn about airgap. To run code in a sandbox, install the airgap binary (see get_install_instructions) and register it with your client.",
  };
}

// SEP-1960 manifest, served at /.well-known/mcp.
export function mcpManifest() {
  return {
    mcp_version: MCP_PROTOCOL_VERSION,
    name: "airgap-docs",
    description: DOCS_MCP_DESCRIPTION,
    documentation_url: `${O}/llms.txt`,
    endpoints: [{ url: MCP_URL, transport: "streamable-http", capabilities: ["tools", "resources"], auth: { type: "none" } }],
    related: [{ name: SITE.registryName, description: "The airgap sandbox server itself (stdio, install locally)", registry: SITE.registryUrl }],
  };
}

export function agentCard() {
  return {
    name: "airgap docs agent",
    description:
      "Answers questions about airgap, the open-source Rust MCP server that runs AI coding agent commands in disposable Linux microVMs on GMI Cloud: which tool to use, install, client setup, credentials and limits. It only answers from airgap's docs; it does not run code.",
    version: SITE.version,
    provider: { organization: "airgap (Dhruvil Mistry)", url: O },
    documentationUrl: `${O}/llms.txt`,
    iconUrl: `${O}/favicon.svg`,
    supportedInterfaces: [{ url: A2A_URL, protocolBinding: "JSONRPC", protocolVersion: "1.0" }],
    // A2A v0.3 fields, kept for clients that predate supportedInterfaces.
    url: A2A_URL,
    preferredTransport: "JSONRPC",
    protocolVersion: "0.3.0",
    capabilities: { streaming: false, pushNotifications: false, extendedAgentCard: false },
    defaultInputModes: ["text/plain"],
    defaultOutputModes: ["text/plain", "application/json"],
    skills: [
      {
        id: "answer-airgap-questions",
        name: "Answer questions about airgap",
        description: "Answers natural-language questions about airgap from its website and docs, with source links.",
        tags: ["airgap", "mcp", "sandbox", "documentation"],
        examples: ["How do I verify a GitHub PR with airgap?", "What credentials does airgap need?", "Does airgap send my files to the cloud?"],
      },
      {
        id: "route-to-airgap-tool",
        name: "Pick the right airgap tool",
        description: "Given a task (verify a PR, test a patch, run tests, scan issue text, audit a manifest, redact logs, security review), names the airgap MCP tool to call and its inputs.",
        tags: ["airgap", "routing", "tools"],
        examples: ["Which airgap tool should I use to test a local diff?", "list airgap tools"],
      },
      {
        id: "install-and-setup",
        name: "Install and set up airgap",
        description: "Returns install commands per platform and setup commands for Claude Code, Cursor, VS Code, Factory Droid or any stdio MCP client.",
        tags: ["airgap", "install", "setup"],
        examples: ["install airgap on windows", "set up airgap for cursor"],
      },
    ],
  };
}

export function ardCatalog() {
  const entry = (name: string, displayName: string, type: string, url: string, description: string, capabilities: string[], representativeQueries: string[]) => ({
    identifier: `urn:air:useairgap.dev:devtools:${name}`,
    displayName,
    description,
    type,
    version: SITE.version,
    url,
    capabilities,
    representativeQueries,
  });
  return {
    entries: [
      entry(
        "airgap-docs-mcp",
        "airgap docs MCP server",
        "application/mcp-server-card+json",
        `${O}/.well-known/mcp/server-card.json`,
        DOCS_MCP_DESCRIPTION,
        DOCS_MCP_TOOLS.map((t) => t.name),
        ["how do I sandbox my coding agent", "verify a GitHub PR in a microVM", "install airgap MCP server"],
      ),
      entry(
        "airgap-docs-agent",
        "airgap docs agent (A2A)",
        "application/a2a-agent-card+json",
        `${O}/.well-known/agent-card.json`,
        "A2A agent that answers questions about airgap from its docs.",
        ["answer-airgap-questions", "route-to-airgap-tool", "install-and-setup"],
        ["which airgap tool tests a local patch", "what does airgap send to GMI Cloud"],
      ),
      entry(
        "airgap-website-api",
        "airgap website API",
        "application/openapi+json",
        `${O}/openapi.json`,
        "Read-only REST API: airgap tools, docs search, install and setup, plus NLWeb /ask.",
        ["listAirgapTools", "getAirgapTool", "searchDocs", "getInstallCommands", "getClientSetup", "askGet"],
        ["list airgap tools", "search airgap docs"],
      ),
      entry(
        "airgap",
        "airgap sandbox MCP server",
        "application/mcp-server-card+json",
        `${O}/agent/airgap-server.json`,
        `${SITE.summary} Runs locally over stdio (airgap --mcp) or as Streamable HTTP (airgap serve); needs your own GMI Cloud keys.`,
        TOOLS.map((t) => t.name),
        ["run tests in a disposable sandbox", "verify a pull request without running it locally", "scan an issue for prompt injection", "redact secrets from logs"],
      ),
      entry("airgap-llms-txt", "airgap llms.txt", "text/markdown", `${O}/llms.txt`, "Index of airgap docs for language models.", ["documentation"], ["airgap documentation"]),
    ],
  };
}

// The sandbox server itself, in the same server.json shape it is published with in the MCP Registry.
export function sandboxServerCard() {
  return {
    $schema: "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json",
    name: SITE.registryName,
    title: "airgap",
    description: "Zero-trust microVM execution boundary for AI coding agents with secret redaction.",
    websiteUrl: O,
    repository: { url: SITE.repo, source: "github" },
    version: SITE.version,
    _meta: {
      "io.modelcontextprotocol.registry/publisher-provided": {
        transports: { stdio: { command: "airgap", args: ["--mcp"] }, streamableHttp: "airgap serve --port 8080 (self-hosted, POST /mcp)" },
        install: Object.fromEntries(INSTALL_COMMANDS.map((i) => [i.platform, i.command])),
        tools: TOOLS.map((t) => ({ name: t.name, description: t.summary })),
        credentials: ["GMI_AGENTBOX_API_KEY", "GMI_MAAS_API_KEY", "GITHUB_TOKEN (optional)"],
      },
    },
  };
}

export function agentFiles(): Record<string, unknown> {
  const ard = ardCatalog();
  return {
    "agent/knowledge.json": knowledgeJson(),
    "agent/airgap-server.json": sandboxServerCard(),
    "openapi.json": openApi(),
    ".well-known/mcp.json": mcpServerJsonCard(),
    ".well-known/mcp/server-card.json": mcpServerCard(),
    ".well-known/mcp-manifest.json": mcpManifest(),
    ".well-known/agent-card.json": agentCard(),
    ".well-known/ard.json": ard,
    ".well-known/ai-catalog.json": ard,
  };
}
