// One Vercel function behind every agent endpoint (see the rewrites in vercel.json): the read-only REST API
// under /api/v1, NLWeb /ask, a stateless docs MCP server at /mcp, an A2A JSON-RPC endpoint at /a2a, and
// agent-friendly 404s. It answers only from /agent/knowledge.json and /agent/corpus.json, which the build
// writes into the static site, and never runs code or calls a model.

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type Obj = Record<string, any>;
type Chunk = { url: string; title: string; section: string; text: string };
type Data = { knowledge: Obj; chunks: Chunk[]; tokens: string[][]; df: Map<string, number> };
type Loader = (origin: string, path: string) => Promise<unknown>;

const SITE_ORIGIN = "https://www.useairgap.dev";
const MCP_VERSIONS = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const MAX_BODY = 64 * 1024;
const MAX_QUERY = 500;

const CORS: Record<string, string> = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, POST, OPTIONS",
  "access-control-allow-headers": "content-type, accept, authorization, mcp-protocol-version, mcp-session-id, last-event-id",
  "access-control-expose-headers": "mcp-protocol-version",
};

const json = (body: unknown, status = 200, headers: Record<string, string> = {}) =>
  new Response(JSON.stringify(body, null, 2) + "\n", {
    status,
    headers: { "content-type": "application/json; charset=utf-8", "cache-control": status === 200 ? "public, max-age=300" : "no-store", "x-content-type-options": "nosniff", ...CORS, ...headers },
  });
const apiError = (status: number, code: string, message: string, headers: Record<string, string> = {}) =>
  json({ error: { code, message, docs: `${SITE_ORIGIN}/openapi.json` } }, status, headers);

// ---------- data ----------
const fetchJson: Loader = async (origin, path) => {
  const res = await fetch(new URL(path, origin), { headers: { accept: "application/json" } });
  if (!res.ok) throw new Error(`${path}: HTTP ${res.status}`);
  return res.json();
};
let cached: Promise<Data> | null = null;

async function loadData(origin: string, load: Loader): Promise<Data> {
  const get = (o: string) => Promise.all([load(o, "/agent/knowledge.json"), load(o, "/agent/corpus.json")]);
  const [knowledge, corpus] = (await get(origin).catch(() => get(SITE_ORIGIN))) as [Obj, { chunks: Chunk[] }];
  const chunks: Chunk[] = [
    ...corpus.chunks,
    ...(knowledge.tools as Obj[]).map((t) => ({
      url: `${SITE_ORIGIN}/docs#tools`,
      title: "airgap MCP tools",
      section: t.name,
      text: `${t.name}: ${t.summary} Use it when: ${t.useWhen} Runs on: ${t.runsOn.join(", ")}. Needs: ${t.needsCredentials}.`,
    })),
  ];
  const tokens = chunks.map((c) => tokenize(`${c.title} ${c.section} ${c.section} ${c.text}`));
  const df = new Map<string, number>();
  for (const doc of tokens) for (const t of new Set(doc)) df.set(t, (df.get(t) ?? 0) + 1);
  return { knowledge, chunks, tokens, df };
}

// ---------- search ----------
const STOP = new Set("a an and are as at be by can do does for from how i if in is it its me my of on or should the to use using what when where which with you your airgap".split(" "));
function tokenize(s: string): string[] {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9_.\-/ ]+/g, " ")
    .split(/[\s/]+/)
    .map((w) => w.replace(/^[.\-]+|[.\-]+$/g, ""))
    .filter((w) => w.length > 1 && !STOP.has(w));
}

function search(data: Data, query: string, limit: number) {
  const q = tokenize(query);
  const n = data.chunks.length;
  const avg = data.tokens.reduce((s, d) => s + d.length, 0) / Math.max(n, 1);
  const scored = data.chunks.map((chunk, i) => {
    const doc = data.tokens[i];
    let score = 0;
    for (const term of new Set(q)) {
      const tf = doc.filter((t) => t === term || (term.length > 3 && t.startsWith(term))).length;
      if (!tf) continue;
      const idf = Math.log(1 + (n - (data.df.get(term) ?? 0) + 0.5) / ((data.df.get(term) ?? 0) + 0.5));
      score += idf * ((tf * 2.2) / (tf + 1.2 * (0.25 + 0.75 * (doc.length / avg))));
    }
    const head = `${chunk.section} ${chunk.title}`.toLowerCase();
    if (q.some((t) => head.includes(t))) score *= 1.3;
    return { chunk, score };
  });
  return scored
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map(({ chunk, score }) => ({ url: chunk.url, title: chunk.title, section: chunk.section, text: chunk.text, score: Math.round(score * 1000) / 1000 }));
}

// ---------- answers shared by MCP, A2A and /ask ----------
const pick = <T extends Obj>(list: T[], key: string, value: unknown) => list.find((x) => x[key] === value);

function installAnswer(k: Obj, platform?: string) {
  const list = platform ? (k.install as Obj[]).filter((i) => i.platform === platform) : (k.install as Obj[]);
  return { install: list, next: ["airgap setup --yes   # or --claude, --cursor, --vscode, --droid", "airgap doctor", "Set GMI_AGENTBOX_API_KEY (sandboxes) and GMI_MAAS_API_KEY (scans and audits)."] };
}

function answer(data: Data, query: string) {
  const k = data.knowledge;
  const q = query.toLowerCase();
  const sources = search(data, query, 4);
  const lines: string[] = [];
  if (/\binstall|download|binary\b/.test(q)) {
    lines.push("Install airgap:", ...(k.install as Obj[]).map((i) => `- ${i.label}: \`${i.command}\``), "", "Then run `airgap setup --yes` and `airgap doctor`.");
  } else if (/\bset ?up|configure|register|claude|cursor|vs ?code|droid|windsurf|mcp\.json\b/.test(q)) {
    lines.push("Register airgap with your MCP client:");
    for (const c of k.clientSetup as Obj[]) lines.push(c.command ? `- ${c.label}: \`${c.command}\`` : `- ${c.label}: \`${JSON.stringify(c.config)}\``);
  } else if (/\bwhich tool|what tool|tools?\b|route|routing/.test(q)) {
    lines.push("Pick the airgap tool by task:", ...(k.routing as Obj[]).map((r) => `- ${r.when}: \`${r.tool}\`. ${r.note}`));
  }
  if (sources.length) {
    if (lines.length) lines.push("");
    lines.push(lines.length ? "Related docs:" : "From the airgap docs:");
    for (const s of sources) lines.push(`- ${s.section} (${s.url}): ${s.text.replace(/\s+/g, " ").slice(0, 320)}${s.text.length > 320 ? "…" : ""}`);
  }
  if (!lines.length) lines.push(`No match in the airgap docs. Start from ${SITE_ORIGIN}/llms.txt or ${SITE_ORIGIN}/docs.`);
  return { text: lines.join("\n"), sources };
}

// ---------- NLWeb /ask ----------
function nlwebItem(s: ReturnType<typeof search>[number]) {
  return {
    "@type": "TechArticle",
    url: s.url,
    name: s.section === s.title ? s.title : `${s.section} · ${s.title}`,
    site: "useairgap.dev",
    score: s.score,
    description: s.text.replace(/\s+/g, " ").slice(0, 400),
    schema_object: { "@context": "https://schema.org", "@type": "TechArticle", headline: s.section, name: s.title, url: s.url, articleBody: s.text, isPartOf: { "@type": "WebSite", name: "airgap", url: `${SITE_ORIGIN}/` } },
  };
}

async function ask(request: Request, url: URL, data: Data) {
  let query = url.searchParams.get("query") ?? url.searchParams.get("q") ?? "";
  let streaming = url.searchParams.get("streaming");
  if (request.method === "POST") {
    const body = await readJson(request);
    if (body instanceof Response) return body;
    const b = body as Obj;
    query = String(b.query?.text ?? b.query ?? b.q ?? query);
    if (b.streaming !== undefined) streaming = String(b.streaming);
  }
  query = query.trim();
  if (!query) return apiError(400, "missing_query", "Pass the question as ?query=... (GET) or {\"query\": \"...\"} (POST).");
  if (query.length > MAX_QUERY) return apiError(400, "query_too_long", `Queries are limited to ${MAX_QUERY} characters.`);
  const results = search(data, query, 8).map(nlwebItem);
  const { text } = answer(data, query);
  const summary = { "@type": "SearchSummary", text };
  const queryId = crypto.randomUUID();
  const wantsStream = /^(1|true|yes)$/i.test(streaming ?? "") || (streaming === null && (request.headers.get("accept") ?? "").includes("text/event-stream"));
  if (!wantsStream) {
    return json({ _meta: { response_type: "answer", version: "0.55", streaming: false }, query_id: queryId, query, summary, results }, 200, { "cache-control": "no-store" });
  }
  const events = [
    ["start", { _meta: { response_type: "answer", version: "0.55", streaming: true }, query_id: queryId, query, message_type: "begin-nlweb-response" }],
    ["result", { index: 0, item: summary, message_type: "summary" }],
    ...results.map((item, i) => ["result", { index: i + 1, item, message_type: "result_batch", results: [item] }]),
    ["complete", { query_id: queryId, total: results.length, message_type: "end-nlweb-response" }],
  ];
  const body = events.map(([event, payload]) => `event: ${event}\ndata: ${JSON.stringify(payload)}\n\n`).join("");
  return new Response(body, { status: 200, headers: { "content-type": "text/event-stream; charset=utf-8", "cache-control": "no-cache", "x-content-type-options": "nosniff", ...CORS } });
}

// ---------- MCP (stateless Streamable HTTP, JSON responses) ----------
const rpcResult = (id: Json, result: unknown) => ({ jsonrpc: "2.0", id, result });
const rpcError = (id: Json, code: number, message: string, data?: unknown) => ({ jsonrpc: "2.0", id, error: data === undefined ? { code, message } : { code, message, data } });
const textResult = (value: unknown, isError = false) => ({
  content: [{ type: "text", text: typeof value === "string" ? value : JSON.stringify(value, null, 2) }],
  ...(typeof value === "object" && value !== null && !isError ? { structuredContent: value } : {}),
  isError,
});

async function mcpCall(data: Data, origin: string, msg: Obj): Promise<Obj | null> {
  const id = msg.id ?? null;
  const isNotification = msg.id === undefined;
  const k = data.knowledge;
  const params = (msg.params ?? {}) as Obj;
  if (msg.jsonrpc !== "2.0" || typeof msg.method !== "string") return isNotification ? null : rpcError(id, -32600, "Invalid Request");
  if (msg.method.startsWith("notifications/")) return null;
  switch (msg.method) {
    case "initialize": {
      const asked = String(params.protocolVersion ?? "");
      return rpcResult(id, {
        protocolVersion: MCP_VERSIONS.includes(asked) ? asked : MCP_VERSIONS[0],
        capabilities: { tools: { listChanged: false }, resources: { listChanged: false, subscribe: false }, prompts: { listChanged: false } },
        serverInfo: { name: "airgap-docs", title: "airgap docs", version: k.site.version, websiteUrl: SITE_ORIGIN },
        instructions:
          "Read-only documentation for airgap. Use `ask` or `search_docs` for questions, `list_airgap_tools` to pick the right airgap tool, and `get_install_instructions` / `get_client_setup` to install it. This server does not run code; the airgap binary does.",
      });
    }
    case "ping":
      return rpcResult(id, {});
    case "tools/list":
      return rpcResult(id, { tools: k.docsMcpTools });
    case "resources/list":
      return rpcResult(id, {
        resources: [
          { uri: `${SITE_ORIGIN}/llms.txt`, name: "llms.txt", title: "airgap llms.txt", mimeType: "text/plain" },
          { uri: `${SITE_ORIGIN}/llms-full.txt`, name: "llms-full.txt", title: "Every airgap page and doc", mimeType: "text/plain" },
          ...(k.pages as Obj[]).map((p) => ({ uri: p.markdownUrl, name: p.markdown.slice(1), title: p.title, mimeType: "text/markdown" })),
        ],
      });
    case "resources/read": {
      const uri = String(params.uri ?? "");
      let path: string;
      try {
        const u = new URL(uri);
        if (u.origin !== SITE_ORIGIN || !/\.(md|txt)$/.test(u.pathname)) throw new Error("outside");
        path = u.pathname;
      } catch {
        return rpcError(id, -32002, "Resource not found", { uri });
      }
      try {
        const res = await fetch(new URL(path, origin));
        if (!res.ok) return rpcError(id, -32002, "Resource not found", { uri });
        return rpcResult(id, { contents: [{ uri, mimeType: path.endsWith(".md") ? "text/markdown" : "text/plain", text: await res.text() }] });
      } catch {
        return rpcError(id, -32603, "Could not read resource", { uri });
      }
    }
    case "prompts/list":
      return rpcResult(id, { prompts: [] });
    case "tools/call": {
      const args = (params.arguments ?? {}) as Obj;
      const name = String(params.name ?? "");
      const query = String(args.query ?? "").trim().slice(0, MAX_QUERY);
      switch (name) {
        case "ask":
          if (!query) return rpcResult(id, textResult("query is required", true));
          return rpcResult(id, textResult(answer(data, query).text));
        case "search_docs": {
          if (!query) return rpcResult(id, textResult("query is required", true));
          const limit = Math.min(Math.max(Number(args.limit) || 5, 1), 20);
          return rpcResult(id, textResult({ query, results: search(data, query, limit) }));
        }
        case "list_airgap_tools":
          return rpcResult(id, textResult({ routing: k.routing, tools: k.tools }));
        case "get_install_instructions": {
          const platform = args.platform ? String(args.platform) : undefined;
          if (platform && !pick(k.install, "platform", platform)) return rpcResult(id, textResult(`Unknown platform "${platform}"`, true));
          return rpcResult(id, textResult(installAnswer(k, platform)));
        }
        case "get_client_setup": {
          const client = args.client ? String(args.client) : undefined;
          const list = client ? (k.clientSetup as Obj[]).filter((c) => c.client === client) : k.clientSetup;
          if (client && !list.length) return rpcResult(id, textResult(`Unknown client "${client}"`, true));
          return rpcResult(id, textResult({ setup: list, then: "airgap doctor" }));
        }
        default:
          return rpcError(id, -32602, `Unknown tool: ${name}`);
      }
    }
    default:
      return isNotification ? null : rpcError(id, -32601, `Method not found: ${msg.method}`);
  }
}

async function mcp(request: Request, data: Data, origin: string) {
  const headers = { "mcp-protocol-version": MCP_VERSIONS[0] };
  if (request.method === "GET" || request.method === "DELETE") {
    // Stateless server: no server-initiated stream and no sessions to end.
    return json({ jsonrpc: "2.0", id: null, error: { code: -32000, message: "Method not allowed. POST JSON-RPC messages to this endpoint." } }, 405, { allow: "POST, OPTIONS", ...headers });
  }
  const body = await readJson(request);
  if (body instanceof Response) return json(rpcError(null, -32700, "Parse error"), 400, headers);
  const batch = Array.isArray(body);
  const messages = (batch ? body : [body]) as Obj[];
  if (!messages.length) return json(rpcError(null, -32600, "Invalid Request"), 400, headers);
  const replies = (await Promise.all(messages.map((m) => (m && typeof m === "object" ? mcpCall(data, origin, m) : Promise.resolve(rpcError(null, -32600, "Invalid Request")))))).filter(Boolean);
  if (!replies.length) return new Response(null, { status: 202, headers: { ...CORS, ...headers } });
  return json(batch ? replies : replies[0], 200, { "cache-control": "no-store", ...headers });
}

// ---------- A2A (JSON-RPC, v1.0 and v0.3 method names) ----------
async function a2a(request: Request, data: Data, origin: string, load: Loader) {
  if (request.method === "GET") return json(await load(origin, "/.well-known/agent-card.json").catch(() => ({ agentCard: `${SITE_ORIGIN}/.well-known/agent-card.json` })));
  const body = await readJson(request);
  if (body instanceof Response) return json(rpcError(null, -32700, "Parse error"), 400);
  const msg = body as Obj;
  const id = msg?.id ?? null;
  if (msg?.jsonrpc !== "2.0" || typeof msg.method !== "string") return json(rpcError(id, -32600, "Invalid Request"), 400);
  const v1 = !msg.method.includes("/");
  switch (msg.method) {
    case "SendMessage":
    case "message/send": {
      const parts = (msg.params?.message?.parts ?? []) as Obj[];
      const query = parts.map((p) => (typeof p.text === "string" ? p.text : "")).join(" ").trim().slice(0, MAX_QUERY);
      if (!query) return json(rpcError(id, -32602, "Invalid params: message.parts must contain text"));
      const { text, sources } = answer(data, query);
      const contextId = msg.params?.message?.contextId ?? crypto.randomUUID();
      const messageId = crypto.randomUUID();
      const meta = { sources: sources.map((s) => s.url) };
      const message = v1
        ? { messageId, contextId, role: "ROLE_AGENT", parts: [{ text }], metadata: meta }
        : { kind: "message", messageId, contextId, role: "agent", parts: [{ kind: "text", text }], metadata: meta };
      return json(rpcResult(id, v1 ? { message } : message), 200, { "cache-control": "no-store" });
    }
    case "GetTask":
    case "tasks/get":
    case "CancelTask":
    case "tasks/cancel":
      return json(rpcError(id, -32001, "Task not found: this agent answers synchronously and keeps no tasks"));
    case "SendStreamingMessage":
    case "message/stream":
    case "SubscribeToTask":
    case "tasks/resubscribe":
      return json(rpcError(id, -32004, "Unsupported operation: streaming is not enabled (see capabilities in the agent card)"));
    default:
      return json(rpcError(id, -32601, `Method not found: ${msg.method}`));
  }
}

// ---------- REST ----------
function rest(route: string, url: URL, data: Data) {
  const k = data.knowledge;
  const path = route.replace(/\/+$/, "");
  if (path === "/api/v1" || path === "/api") {
    return json({
      name: "airgap website API",
      version: k.site.version,
      description: "Read-only facts about airgap. Unauthenticated. Errors are JSON.",
      openapi: `${SITE_ORIGIN}/openapi.json`,
      endpoints: [
        { method: "GET", path: "/api/v1/health" },
        { method: "GET", path: "/api/v1/tools" },
        { method: "GET", path: "/api/v1/tools/{name}" },
        { method: "GET", path: "/api/v1/search?q={text}&limit={n}" },
        { method: "GET", path: "/api/v1/install?platform={macos|linux|windows|cargo}" },
        { method: "GET", path: "/api/v1/setup?client={claude-code|cursor|vscode|factory-droid|other}" },
        { method: "GET|POST", path: "/ask?query={text}" },
        { method: "POST", path: "/mcp" },
        { method: "POST", path: "/a2a" },
      ],
    });
  }
  if (path === "/api/v1/health") return json({ status: "ok", service: "airgap-website", version: k.site.version }, 200, { "cache-control": "no-store" });
  if (path === "/api/v1/tools") return json({ tools: k.tools, routing: k.routing });
  const tool = /^\/api\/v1\/tools\/([^/]+)$/.exec(path);
  if (tool) {
    const hit = pick(k.tools, "name", decodeURIComponent(tool[1]));
    return hit ? json(hit) : apiError(404, "tool_not_found", `No airgap tool named "${decodeURIComponent(tool[1])}". GET /api/v1/tools lists them.`);
  }
  if (path === "/api/v1/search") {
    const q = (url.searchParams.get("q") ?? url.searchParams.get("query") ?? "").trim();
    if (!q) return apiError(400, "missing_query", "Pass the search text as ?q=...");
    if (q.length > MAX_QUERY) return apiError(400, "query_too_long", `Queries are limited to ${MAX_QUERY} characters.`);
    const limit = Math.min(Math.max(Number(url.searchParams.get("limit")) || 5, 1), 20);
    return json({ query: q, results: search(data, q, limit) });
  }
  if (path === "/api/v1/install") {
    const platform = url.searchParams.get("platform") ?? undefined;
    if (platform && !pick(k.install, "platform", platform)) return apiError(404, "unknown_platform", "platform must be one of macos, linux, windows, cargo.");
    return json(installAnswer(k, platform));
  }
  if (path === "/api/v1/setup") {
    const client = url.searchParams.get("client");
    const list = client ? (k.clientSetup as Obj[]).filter((c) => c.client === client) : k.clientSetup;
    if (client && !list.length) return apiError(404, "unknown_client", "client must be one of claude-code, cursor, vscode, factory-droid, other.");
    return json({ setup: list, then: "airgap doctor" });
  }
  return apiError(404, "not_found", `No API endpoint at ${path}. GET /api/v1 lists the endpoints.`);
}

function notFound(request: Request, path: string) {
  const accept = request.headers.get("accept") ?? "";
  if (accept.includes("application/json") && !accept.includes("text/markdown")) return apiError(404, "not_found", `Nothing at ${path}.`);
  const md = `---\ntitle: Page not found\nstatus: 404\n---\n\n# Page not found\n\nThere is nothing at \`${path.replace(/`/g, "")}\` on useairgap.dev.\n\n## Try these instead\n\n- [Home (Markdown)](${SITE_ORIGIN}/index.md)\n- [llms.txt](${SITE_ORIGIN}/llms.txt): index of every page and doc\n- [Docs](${SITE_ORIGIN}/docs.md)\n- [Sitemap](${SITE_ORIGIN}/sitemap.xml)\n- Search: \`GET ${SITE_ORIGIN}/api/v1/search?q=...\`\n- Ask: \`GET ${SITE_ORIGIN}/ask?query=...\`\n`;
  return new Response(md, { status: 404, headers: { "content-type": "text/markdown; charset=utf-8", "cache-control": "no-store", "x-content-type-options": "nosniff", ...CORS } });
}

// ---------- entry ----------
async function readJson(request: Request): Promise<unknown | Response> {
  const text = await request.text();
  if (text.length > MAX_BODY) return apiError(413, "body_too_large", `Request bodies are limited to ${MAX_BODY} bytes.`);
  try {
    return JSON.parse(text);
  } catch {
    return apiError(400, "invalid_json", "The request body is not valid JSON.");
  }
}

export async function handle(request: Request, load: Loader = fetchJson): Promise<Response> {
  const url = new URL(request.url);
  const route = url.searchParams.get("route") ?? url.pathname;
  url.searchParams.delete("route");
  if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: { ...CORS, "access-control-max-age": "86400" } });

  if (route.startsWith("/__not_found")) return notFound(request, route.slice("/__not_found".length) || "/");

  let data: Data;
  try {
    if (load === fetchJson) cached ??= loadData(url.origin, load).catch((e) => ((cached = null), Promise.reject(e)));
    data = await (load === fetchJson ? cached! : loadData(url.origin, load));
  } catch {
    return apiError(503, "data_unavailable", "The docs index could not be loaded. Try again shortly.", { "retry-after": "30" });
  }

  try {
    if (route === "/ask") return request.method === "GET" || request.method === "POST" ? await ask(request, url, data) : apiError(405, "method_not_allowed", "Use GET or POST.", { allow: "GET, POST, OPTIONS" });
    if (route === "/mcp") return await mcp(request, data, url.origin);
    if (route === "/a2a") return await a2a(request, data, url.origin, load);
    if (route === "/api" || route.startsWith("/api/")) {
      if (request.method !== "GET" && request.method !== "HEAD") return apiError(405, "method_not_allowed", "This API is read-only; use GET.", { allow: "GET, OPTIONS" });
      return rest(route, url, data);
    }
    return notFound(request, route);
  } catch {
    return apiError(500, "internal_error", "Unexpected error while answering. The docs are also at /llms.txt.");
  }
}

export const GET = (request: Request) => handle(request);
export const POST = (request: Request) => handle(request);
export const DELETE = (request: Request) => handle(request);
export const OPTIONS = (request: Request) => handle(request);
export const HEAD = (request: Request) => handle(request);
