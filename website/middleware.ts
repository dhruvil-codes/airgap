// Vercel Routing Middleware: AI agents acting for a user (and any client whose Accept header prefers
// Markdown) get each page's Markdown twin instead of the HTML app shell. Browsers and search crawlers
// are untouched. Every matched page has a prerendered `<path>.md` (the homepage uses /index.md).
export const config = {
  matcher: ["/", "/docs", "/guides", "/about", "/contact", "/privacy", "/terms", "/conditions"],
};

const AGENT_USER_AGENTS = /\b(ChatGPT-User|Claude-User|Perplexity-User|MistralAI-User|DuckAssistBot|Google-Agent|Gemini-User|meta-externalfetcher)\b/i;

function quality(accept: string, types: string[]) {
  let best = 0;
  let order = Infinity;
  accept.split(",").forEach((part, i) => {
    const [type, ...params] = part.trim().toLowerCase().split(";");
    if (!types.includes(type.trim())) return;
    const q = params.map((p) => p.trim()).find((p) => p.startsWith("q="));
    const value = q ? Number(q.slice(2)) || 0 : 1;
    if (value > best) [best, order] = [value, i];
  });
  return { q: best, order };
}

export function prefersMarkdown(request: Request) {
  if (AGENT_USER_AGENTS.test(request.headers.get("user-agent") ?? "")) return true;
  const accept = request.headers.get("accept") ?? "";
  const md = quality(accept, ["text/markdown", "text/x-markdown"]);
  const html = quality(accept, ["text/html", "application/xhtml+xml"]);
  return md.q > 0 && (md.q > html.q || (md.q === html.q && md.order < html.order));
}

export function markdownPath(pathname: string) {
  const path = pathname.replace(/\/+$/, "") || "/";
  return path === "/" ? "/index.md" : `${path}.md`;
}

export default function middleware(request: Request) {
  if (request.method !== "GET" && request.method !== "HEAD") return;
  if (!prefersMarkdown(request)) return;
  const url = new URL(request.url);
  return new Response(null, { headers: { "x-middleware-rewrite": new URL(markdownPath(url.pathname), url).toString() } });
}
