// Checks the agent-facing surface against the built site: run after `npm run build`.
// Calls the real Vercel function handler and middleware in-process, with static files served from dist/.
import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { handle } from "../api/router.ts";
import middleware, { markdownPath } from "../middleware.ts";

const site = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = join(site, "dist");
const O = "https://www.useairgap.dev";
const readDist = (p) => readFile(join(dist, decodeURIComponent(p)), "utf8");
const exists = (p) => stat(p).then(() => true, () => false);

// Same-origin fetches from the function (data files, MCP resources/read) are served from dist/.
globalThis.fetch = async (input) => {
  const url = new URL(String(input instanceof Request ? input.url : input));
  if (url.origin !== O) throw new Error(`unexpected fetch ${url}`);
  try {
    return new Response(await readDist(url.pathname), { status: 200 });
  } catch {
    return new Response("not found", { status: 404 });
  }
};

let passed = 0;
const failures = [];
async function test(name, fn) {
  try {
    await fn();
    passed++;
  } catch (err) {
    failures.push(`${name}: ${err.message}`);
  }
}
const call = (path, init = {}) => handle(new Request(new URL(path, O), init));
const rpc = (route, body, headers = {}) => call(`/api/router?route=${route}`, { method: "POST", headers: { "content-type": "application/json", accept: "application/json, text/event-stream", ...headers }, body: JSON.stringify(body) });
const isJsonError = async (res, status) => {
  assert.equal(res.status, status);
  assert.match(res.headers.get("content-type"), /application\/json/);
  const body = await res.json();
  assert.equal(typeof body.error?.code, "string");
  assert.equal(typeof body.error?.message, "string");
};

// ---------- REST ----------
await test("GET /api/v1 lists endpoints", async () => {
  const res = await call("/api/router?route=/api/v1");
  assert.equal(res.status, 200);
  assert.ok((await res.json()).endpoints.length >= 6);
});
await test("GET /api/v1/health", async () => assert.equal((await (await call("/api/router?route=/api/v1/health")).json()).status, "ok"));
await test("GET /api/v1/tools returns all 8 tools with schemas", async () => {
  const { tools } = await (await call("/api/router?route=/api/v1/tools")).json();
  assert.equal(tools.length, 8);
  for (const t of tools) assert.equal(t.inputSchema.type, "object");
});
await test("GET /api/v1/tools/{name}", async () => assert.equal((await (await call("/api/router?route=/api/v1/tools/airgap_verify_pr")).json()).name, "airgap_verify_pr"));
await test("unknown tool is a JSON 404", async () => isJsonError(await call("/api/router?route=/api/v1/tools/nope"), 404));
await test("unknown API path is a JSON 404", async () => isJsonError(await call("/api/router?route=/api/does-not-exist"), 404));
await test("writes to the API are a JSON 405", async () => isJsonError(await call("/api/router?route=/api/v1/tools", { method: "POST", body: "{}" }), 405));
await test("search without q is a JSON 400", async () => isJsonError(await call("/api/router?route=/api/v1/search"), 400));
await test("search finds PR verification docs", async () => {
  const { results } = await (await call("/api/router?route=/api/v1/search&q=verify%20a%20github%20pull%20request")).json();
  assert.ok(results.length > 0);
  assert.ok(results.slice(0, 3).some((r) => /verify_pr|pull request|PR/i.test(r.section + r.text)), JSON.stringify(results.slice(0, 3).map((r) => r.section)));
  for (const r of results) assert.ok(r.url.startsWith(O));
});
await test("install and setup lookups", async () => {
  const win = await (await call("/api/router?route=/api/v1/install&platform=windows")).json();
  assert.match(win.install[0].command, /install\.ps1/);
  await isJsonError(await call("/api/router?route=/api/v1/install&platform=amiga"), 404);
  const cursor = await (await call("/api/router?route=/api/v1/setup&client=cursor")).json();
  assert.equal(cursor.setup[0].command, "airgap setup --cursor --yes");
});
await test("CORS preflight", async () => {
  const res = await call("/api/router?route=/mcp", { method: "OPTIONS" });
  assert.equal(res.status, 204);
  assert.equal(res.headers.get("access-control-allow-origin"), "*");
});

// ---------- NLWeb /ask ----------
await test("GET /ask returns NLWeb JSON with schema.org results", async () => {
  const res = await call("/api/router?route=/ask&query=what%20does%20airgap%20send%20to%20the%20cloud&streaming=false");
  assert.equal(res.status, 200);
  const body = await res.json();
  assert.equal(typeof body.query_id, "string");
  assert.ok(body.results.length > 0);
  for (const r of body.results) {
    assert.ok(r.url && r.name && r.site);
    assert.equal(r.schema_object["@type"], "TechArticle");
  }
});
await test("POST /ask", async () => {
  const res = await rpc("/ask", { query: "how do I install airgap on windows" }, { accept: "application/json" });
  const body = await res.json();
  assert.ok(body.results.length > 0);
  assert.match(body.summary.text, /install\.ps1/);
});
await test("GET /ask?streaming=true streams SSE", async () => {
  const res = await call("/api/router?route=/ask&query=secret%20redaction&streaming=true");
  assert.match(res.headers.get("content-type"), /text\/event-stream/);
  const body = await res.text();
  assert.match(body, /^event: start\ndata: /);
  assert.match(body, /event: result\ndata: /);
  assert.match(body, /event: complete\ndata: .*\n\n$/);
});
await test("POST /ask with Accept: text/event-stream streams", async () => {
  const res = await call("/api/router?route=/ask", { method: "POST", headers: { accept: "text/event-stream", "content-type": "application/json" }, body: JSON.stringify({ query: "sandbox limits" }) });
  assert.match(res.headers.get("content-type"), /text\/event-stream/);
});
await test("/ask without a query is a JSON 400", async () => isJsonError(await call("/api/router?route=/ask"), 400));

// ---------- MCP ----------
await test("MCP initialize negotiates a version", async () => {
  const res = await rpc("/mcp", { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "t", version: "1" } } });
  const { result } = await res.json();
  assert.equal(result.protocolVersion, "2025-06-18");
  assert.equal(result.serverInfo.name, "airgap-docs");
  assert.ok(result.capabilities.tools);
  const unknown = await (await rpc("/mcp", { jsonrpc: "2.0", id: 2, method: "initialize", params: { protocolVersion: "1999-01-01" } })).json();
  assert.equal(unknown.result.protocolVersion, "2025-11-25");
});
await test("MCP notification gets 202 with no body", async () => {
  const res = await rpc("/mcp", { jsonrpc: "2.0", method: "notifications/initialized" });
  assert.equal(res.status, 202);
  assert.equal(await res.text(), "");
});
await test("MCP tools/list and every tool call", async () => {
  const { result } = await (await rpc("/mcp", { jsonrpc: "2.0", id: 3, method: "tools/list" })).json();
  const names = result.tools.map((t) => t.name);
  assert.deepEqual(names, ["ask", "search_docs", "list_airgap_tools", "get_install_instructions", "get_client_setup"]);
  for (const t of result.tools) assert.equal(t.annotations.readOnlyHint, true);
  const calls = {
    ask: { query: "which tool tests a local patch" },
    search_docs: { query: "receipts", limit: 3 },
    list_airgap_tools: {},
    get_install_instructions: { platform: "macos" },
    get_client_setup: { client: "claude-code" },
  };
  for (const [name, args] of Object.entries(calls)) {
    const out = await (await rpc("/mcp", { jsonrpc: "2.0", id: name, method: "tools/call", params: { name, arguments: args } })).json();
    assert.equal(out.result.isError, false, `${name}: ${JSON.stringify(out)}`);
    assert.ok(out.result.content[0].text.length > 20, name);
  }
  const ask = await (await rpc("/mcp", { jsonrpc: "2.0", id: 9, method: "tools/call", params: { name: "ask", arguments: { query: "which tool tests a local patch" } } })).json();
  assert.match(ask.result.content[0].text, /airgap_verify_patch/);
});
await test("MCP errors", async () => {
  const bad = await (await rpc("/mcp", { jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "rm_rf" } })).json();
  assert.equal(bad.error.code, -32602);
  const missing = await (await rpc("/mcp", { jsonrpc: "2.0", id: 5, method: "nope" })).json();
  assert.equal(missing.error.code, -32601);
  const parse = await call("/api/router?route=/mcp", { method: "POST", headers: { "content-type": "application/json" }, body: "{" });
  assert.equal((await parse.json()).error.code, -32700);
  const get = await call("/api/router?route=/mcp");
  assert.equal(get.status, 405);
});
await test("MCP batch and resources", async () => {
  const out = await (await rpc("/mcp", [{ jsonrpc: "2.0", id: 1, method: "ping" }, { jsonrpc: "2.0", method: "notifications/initialized" }, { jsonrpc: "2.0", id: 2, method: "resources/list" }])).json();
  assert.equal(out.length, 2);
  const uri = out[1].result.resources[0].uri;
  const read = await (await rpc("/mcp", { jsonrpc: "2.0", id: 3, method: "resources/read", params: { uri } })).json();
  assert.match(read.result.contents[0].text, /^# airgap/);
  const outside = await (await rpc("/mcp", { jsonrpc: "2.0", id: 4, method: "resources/read", params: { uri: "https://evil.example/x.md" } })).json();
  assert.equal(outside.error.code, -32002);
});

// ---------- A2A ----------
await test("A2A SendMessage (v1.0)", async () => {
  const out = await (await rpc("/a2a", { jsonrpc: "2.0", id: 1, method: "SendMessage", params: { message: { messageId: "m1", role: "ROLE_USER", parts: [{ text: "how do I set up airgap for cursor?" }] } } })).json();
  assert.equal(out.result.message.role, "ROLE_AGENT");
  assert.match(out.result.message.parts[0].text, /airgap setup --cursor/);
});
await test("A2A message/send (v0.3)", async () => {
  const out = await (await rpc("/a2a", { jsonrpc: "2.0", id: 2, method: "message/send", params: { message: { kind: "message", messageId: "m2", role: "user", parts: [{ kind: "text", text: "does airgap send my files to the cloud" }] } } })).json();
  assert.equal(out.result.kind, "message");
  assert.equal(out.result.parts[0].kind, "text");
});
await test("A2A unsupported methods", async () => {
  assert.equal((await (await rpc("/a2a", { jsonrpc: "2.0", id: 3, method: "tasks/get", params: { id: "x" } })).json()).error.code, -32001);
  assert.equal((await (await rpc("/a2a", { jsonrpc: "2.0", id: 4, method: "message/stream", params: {} })).json()).error.code, -32004);
});

// ---------- 404s ----------
await test("unknown page as Markdown for agents", async () => {
  const res = await call("/api/router?route=/__not_found/nope", { headers: { accept: "text/markdown" } });
  assert.equal(res.status, 404);
  assert.match(res.headers.get("content-type"), /text\/markdown/);
  assert.match(await res.text(), /^---\ntitle: Page not found/);
  await isJsonError(await call("/api/router?route=/__not_found/nope", { headers: { accept: "application/json" } }), 404);
});

// ---------- middleware ----------
const mw = (path, headers) => middleware(new Request(new URL(path, O), { headers }));
await test("middleware serves Markdown to user agents and Markdown Accept", async () => {
  assert.equal(mw("/", { "user-agent": "Mozilla/5.0 (compatible; Claude-User/1.0)" })?.headers.get("x-middleware-rewrite"), `${O}/index.md`);
  assert.equal(mw("/docs", { "user-agent": "Mozilla/5.0 ChatGPT-User/1.0" })?.headers.get("x-middleware-rewrite"), `${O}/docs.md`);
  assert.equal(mw("/guides/", { "user-agent": "Perplexity-User/1.0" })?.headers.get("x-middleware-rewrite"), `${O}/guides.md`);
  assert.equal(mw("/terms", { accept: "text/markdown, text/html;q=0.8" })?.headers.get("x-middleware-rewrite"), `${O}/terms.md`);
  assert.equal(mw("/", { accept: "text/html,application/xhtml+xml,*/*;q=0.8", "user-agent": "Mozilla/5.0 Chrome/140" }), undefined);
  assert.equal(mw("/", { "user-agent": "Googlebot/2.1", accept: "text/html" }), undefined);
  assert.equal(mw("/", { accept: "text/html, text/markdown;q=0.5" }), undefined);
});
await test("every middleware path has a Markdown twin", async () => {
  const { config } = await import("../middleware.ts");
  for (const p of config.matcher) assert.ok(await exists(join(dist, markdownPath(p))), markdownPath(p));
});

// ---------- static files ----------
const knowledge = JSON.parse(await readDist("/agent/knowledge.json"));
await test("pages are prerendered with real content and per-page heads", async () => {
  for (const page of knowledge.pages) {
    const html = await readDist(page.path === "/" ? "/index.html" : `${page.path}/index.html`);
    const root = html.split('<div id="root">')[1].split("</body>")[0];
    const text = root.replace(/<script[\s\S]*?<\/script>/g, "").replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");
    assert.ok(text.length > 1500, `${page.path}: only ${text.length} chars of text`);
    assert.ok(html.includes(`<title>${page.title.replace(/&/g, "&amp;")}</title>`), `${page.path}: title`);
    assert.ok(html.includes(`<link rel="canonical" href="${page.url}"`), `${page.path}: canonical`);
    assert.ok(root.includes('href="/docs"'), `${page.path}: links to /docs`);
    for (const m of html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)) JSON.parse(m[1]);
  }
  const nf = await readDist("/404.html");
  assert.match(nf, /noindex/);
  assert.match(nf, /Page not found/);
});
await test("homepage JSON-LD has sameAs and keywords", async () => {
  const html = await readDist("/index.html");
  const ld = JSON.parse(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/.exec(html)[1]);
  const app = ld["@graph"].find((n) => n["@type"] === "SoftwareApplication");
  assert.ok(app.sameAs.includes("https://github.com/dhruvil-codes/airgap"));
  assert.match(html, /<meta name="keywords"/);
  assert.match(html, /rel="ard"/);
});
await test("discovery documents", async () => {
  const card = JSON.parse(await readDist("/.well-known/agent-card.json"));
  for (const k of ["name", "description", "version", "supportedInterfaces", "capabilities", "defaultInputModes", "defaultOutputModes", "skills"]) assert.ok(card[k] !== undefined, `agent-card.${k}`);
  for (const s of card.skills) assert.ok(s.id && s.name && s.description && s.tags.length, "skill fields");
  const mcp = JSON.parse(await readDist("/.well-known/mcp.json"));
  assert.ok(mcp.name && mcp.description.length <= 100 && mcp.version);
  assert.equal(mcp.remotes[0].url, `${O}/mcp`);
  const manifest = JSON.parse(await readDist("/.well-known/mcp-manifest.json"));
  assert.ok(manifest.mcp_version && manifest.endpoints[0].url);
  JSON.parse(await readDist("/.well-known/mcp/server-card.json"));
  const ard = JSON.parse(await readDist("/.well-known/ard.json"));
  for (const e of ard.entries) {
    assert.match(e.identifier, /^urn:air:useairgap\.dev:/);
    assert.ok(e.url && e.type && e.displayName);
    const u = new URL(e.url);
    if (u.origin === O) assert.ok(await exists(join(dist, u.pathname)), e.url);
  }
  assert.deepEqual(JSON.parse(await readDist("/.well-known/ai-catalog.json")), ard);
});
await test("OpenAPI spec: every operation has an operationId and is served by the handler", async () => {
  const spec = JSON.parse(await readDist("/openapi.json"));
  assert.equal(spec.openapi, "3.1.0");
  const ids = new Set();
  for (const [path, item] of Object.entries(spec.paths)) {
    for (const [method, op] of Object.entries(item)) {
      assert.ok(op.operationId, `${method} ${path}`);
      assert.ok(!ids.has(op.operationId), `duplicate ${op.operationId}`);
      ids.add(op.operationId);
      const concrete = path.replace("{name}", "airgap_sandbox_run");
      const qs = { "/api/v1/search": "&q=sandbox", "/ask": "&query=sandbox&streaming=false" }[path] ?? "";
      const init = method === "post" ? { method: "POST", headers: { "content-type": "application/json", accept: "application/json" }, body: JSON.stringify({ query: "sandbox" }) } : {};
      const res = await call(`/api/router?route=${concrete}${qs}`, init);
      assert.equal(res.status, 200, `${method.toUpperCase()} ${concrete}`);
    }
  }
});
await test("sitemap URLs all exist in dist", async () => {
  const xml = await readDist("/sitemap.xml");
  const locs = [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => new URL(m[1]).pathname);
  assert.ok(locs.length >= 20);
  for (const p of locs) assert.ok((await exists(join(dist, p))) || (await exists(join(dist, p, "index.html"))), p);
});
await test("Markdown twins have frontmatter and balanced code fences", async () => {
  for (const page of knowledge.pages) {
    const md = await readDist(page.markdown);
    assert.match(md, /^---\ntitle: .+\ndescription: .+/m, page.markdown);
    assert.equal((md.match(/^```/gm) ?? []).length % 2, 0, `${page.markdown}: unbalanced fences`);
  }
});
await test("llms.txt is well-formed and every link resolves", async () => {
  const llms = await readDist("/llms.txt");
  assert.match(llms, /^# airgap\n\n> /);
  assert.match(llms, /## When to use airgap/);
  for (const [, href] of llms.matchAll(/\]\((https:\/\/www\.useairgap\.dev[^)]*)\)/g)) {
    const p = new URL(href).pathname;
    const dynamic = /^\/(api\/v1|ask|mcp)$/.test(p);
    if (dynamic) {
      const res = await call(`/api/router?route=${p}${p === "/ask" ? "&query=airgap&streaming=false" : ""}`, p === "/mcp" ? { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "ping" }) } : {});
      assert.equal(res.status, 200, href);
    } else assert.ok((await exists(join(dist, p))) || (await exists(join(dist, p, "index.html"))), href);
  }
});
await test("vercel.json rewrites only target the router or real files", async () => {
  const vercel = JSON.parse(await readFile(join(site, "vercel.json"), "utf8"));
  for (const r of vercel.rewrites) {
    if (r.destination.startsWith("/api/router?route=")) continue;
    const page = r.source.match(/^\/:page\(([^)]+)\)$/);
    if (page) {
      for (const name of page[1].split("|")) assert.ok(await exists(join(dist, r.destination.replace(":page", name))), `${r.destination} (${name})`);
      continue;
    }
    assert.ok(await exists(join(dist, r.destination)), r.destination);
  }
});

console.log(`${passed} passed, ${failures.length} failed`);
if (failures.length) {
  for (const f of failures) console.log(`FAIL ${f}`);
  process.exit(1);
}
