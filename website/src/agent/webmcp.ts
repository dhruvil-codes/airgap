import { CLIENT_SETUP, INSTALL_COMMANDS, PAGES, ROUTING, SITE, TOOLS } from "./knowledge";
import { at } from "../content";

type ToolResult = { content: { type: "text"; text: string }[]; isError?: boolean };
type WebMcpTool = {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  annotations?: { readOnlyHint?: boolean };
  execute: (args: Record<string, unknown>) => Promise<ToolResult> | ToolResult;
};
type ModelContext = {
  registerTool?: (tool: WebMcpTool) => unknown;
  provideContext?: (context: { tools: WebMcpTool[] }) => unknown;
};

const reply = (value: unknown): ToolResult => ({
  content: [{ type: "text", text: typeof value === "string" ? value : JSON.stringify(value, null, 2) }],
});
const fail = (message: string): ToolResult => ({ ...reply(message), isError: true });

const TOOLS_FOR_PAGE: WebMcpTool[] = [
  {
    name: "airgap_search_docs",
    description: `Search the ${SITE.name} website and docs. Returns the best matching sections with their URLs.`,
    inputSchema: {
      type: "object",
      required: ["query"],
      properties: { query: { type: "string", description: "What to look for, e.g. 'verify a pull request'." }, limit: { type: "integer", minimum: 1, maximum: 10 } },
    },
    annotations: { readOnlyHint: true },
    async execute(args) {
      const query = String(args.query ?? "").trim();
      if (!query) return fail("query is required");
      const limit = Math.min(Math.max(Number(args.limit) || 5, 1), 10);
      try {
        const res = await fetch(at(`/api/v1/search?q=${encodeURIComponent(query)}&limit=${limit}`), { headers: { accept: "application/json" } });
        if (res.ok) return reply(await res.json());
      } catch {
        // Fall back to the tool list below when the API is unreachable (e.g. the local dev server).
      }
      const words = query.toLowerCase().split(/\W+/).filter(Boolean);
      const hits = TOOLS.filter((t) => words.some((w) => `${t.name} ${t.summary} ${t.useWhen}`.toLowerCase().includes(w)));
      return reply({ query, results: hits.slice(0, limit).map((t) => ({ name: t.name, summary: t.summary, url: `${SITE.origin}/docs#tools` })) });
    },
  },
  {
    name: "airgap_get_install_command",
    description: "Get the command that installs the airgap binary on a given platform.",
    inputSchema: {
      type: "object",
      required: ["platform"],
      properties: { platform: { type: "string", enum: INSTALL_COMMANDS.map((i) => i.platform) } },
    },
    annotations: { readOnlyHint: true },
    execute(args) {
      const hit = INSTALL_COMMANDS.find((i) => i.platform === args.platform);
      return hit ? reply({ ...hit, next: "Then run: airgap setup --yes && airgap doctor" }) : fail(`Unknown platform. Use one of: ${INSTALL_COMMANDS.map((i) => i.platform).join(", ")}`);
    },
  },
  {
    name: "airgap_get_client_setup",
    description: "Get the setup command or MCP JSON config that registers airgap with an MCP client.",
    inputSchema: {
      type: "object",
      required: ["client"],
      properties: { client: { type: "string", enum: CLIENT_SETUP.map((c) => c.client) } },
    },
    annotations: { readOnlyHint: true },
    execute(args) {
      const hit = CLIENT_SETUP.find((c) => c.client === args.client);
      return hit ? reply(hit) : fail(`Unknown client. Use one of: ${CLIENT_SETUP.map((c) => c.client).join(", ")}`);
    },
  },
  {
    name: "airgap_list_mcp_tools",
    description: "List the 8 MCP tools the airgap server provides, with when to use each one and its input schema.",
    inputSchema: { type: "object", properties: {} },
    annotations: { readOnlyHint: true },
    execute: () => reply({ routing: ROUTING, tools: TOOLS }),
  },
  {
    name: "airgap_open_page",
    description: "Navigate this tab to a page of the airgap website, optionally scrolled to a section id.",
    inputSchema: {
      type: "object",
      required: ["page"],
      properties: {
        page: { type: "string", enum: PAGES.map((p) => p.path) },
        section: { type: "string", description: "Optional section id, e.g. 'tools' or 'faq'." },
      },
    },
    execute(args) {
      const page = PAGES.find((p) => p.path === args.page);
      if (!page) return fail(`Unknown page. Use one of: ${PAGES.map((p) => p.path).join(", ")}`);
      const hash = typeof args.section === "string" && args.section ? `#${encodeURIComponent(args.section)}` : "";
      window.location.assign(at(page.path) + hash);
      return reply(`Opening ${page.title}`);
    },
  },
];

export function registerWebMcpTools() {
  const mc = (navigator as Navigator & { modelContext?: ModelContext }).modelContext;
  if (!mc) return;
  try {
    if (typeof mc.registerTool === "function") TOOLS_FOR_PAGE.forEach((tool) => mc.registerTool?.(tool));
    else mc.provideContext?.({ tools: TOOLS_FOR_PAGE });
  } catch (err) {
    console.warn("WebMCP registration failed", err);
  }
}
