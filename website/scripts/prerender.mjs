// Runs after `vite build` and `vite build --ssr`. Fills every HTML shell in dist/ with the server-rendered
// page, writes a Markdown twin of each page, mirrors the repo docs, and generates the sitemap, search corpus,
// discovery documents and llms-full.txt. Fails the build if an llms.txt link would not resolve.
import { cp, mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const site = resolve(here, "..");
const dist = join(site, "dist");
const repo = resolve(site, "..");
const ssr = await import(pathToFileURL(join(site, ".ssr", "entry-server.js")).href);
const { render, PAGES, SITE, agentFiles } = ssr;
const O = SITE.origin;
const today = new Date().toISOString().slice(0, 10);

const write = async (rel, body) => {
  const file = join(dist, rel);
  await mkdir(dirname(file), { recursive: true });
  await writeFile(file, body);
};
const exists = (p) => stat(p).then(() => true, () => false);
const esc = (s) => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

// ---------- HTML -> tree -> Markdown (for the markup our own components emit) ----------
const VOID = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"]);
const decode = (s) =>
  s
    .replace(/&nbsp;/g, " ")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;|&#39;/g, "'")
    .replace(/&#x([0-9a-f]+);/gi, (_, h) => String.fromCodePoint(parseInt(h, 16)))
    .replace(/&#(\d+);/g, (_, d) => String.fromCodePoint(Number(d)))
    .replace(/&amp;/g, "&");

function parse(html) {
  const root = { tag: "#root", attrs: {}, children: [] };
  const stack = [root];
  const re = /<!--[\s\S]*?-->|<\/([a-zA-Z0-9-]+)\s*>|<([a-zA-Z0-9-]+)((?:\s+[^\s=>/]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+))?)*)\s*(\/?)>|([^<]+)/g;
  let m;
  while ((m = re.exec(html))) {
    const top = stack[stack.length - 1];
    if (m[5] !== undefined) top.children.push({ text: decode(m[5]) });
    else if (m[1]) {
      const tag = m[1].toLowerCase();
      for (let i = stack.length - 1; i > 0; i--) if (stack[i].tag === tag) { stack.length = i; break; }
    } else if (m[2]) {
      const attrs = {};
      for (const a of m[3].matchAll(/([^\s=>/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?/g)) attrs[a[1].toLowerCase()] = decode(a[2] ?? a[3] ?? a[4] ?? "");
      const node = { tag: m[2].toLowerCase(), attrs, children: [] };
      top.children.push(node);
      if (!VOID.has(node.tag) && !m[4]) stack.push(node);
    }
  }
  return root;
}
const cls = (n) => ` ${n.attrs?.class ?? ""} `;
const find = (n, pred) => {
  if (pred(n)) return n;
  for (const c of n.children ?? []) { const hit = c.tag && find(c, pred); if (hit) return hit; }
  return null;
};
const textOf = (n) => (n.text !== undefined ? n.text : (n.children ?? []).map(textOf).join(""));
const skip = (n) =>
  n.tag && (["button", "svg", "script", "style", "nav", "img"].includes(n.tag) || n.attrs["aria-hidden"] === "true" || /\s(badge|doc-code-copy|ticks)\s/.test(cls(n)));
const BLOCK = new Set(["p", "div", "section", "article", "ul", "ol", "li", "pre", "table", "h1", "h2", "h3", "h4", "h5", "h6", "main", "header", "footer", "blockquote"]);

function absolute(href, pageUrl) {
  if (!href) return href;
  if (href.startsWith("#")) return pageUrl + href;
  if (href.startsWith("/")) return O + href;
  return href;
}
function inline(nodes, pageUrl) {
  let out = "";
  for (const n of nodes) {
    if (n.text !== undefined) { out += n.text.replace(/\s+/g, " "); continue; }
    if (skip(n)) continue;
    const inner = () => inline(n.children, pageUrl);
    if (n.tag === "code") out += "`" + textOf(n) + "`";
    else if (n.tag === "a") { const t = inner().trim(); out += t ? `[${t}](${absolute(n.attrs.href, pageUrl)})` : ""; }
    else if (n.tag === "strong" || n.tag === "b") out += `**${inner().trim()}**`;
    else if (n.tag === "em" || n.tag === "i") out += `_${inner().trim()}_`;
    else if (n.tag === "br") out += "\n";
    else out += inner();
  }
  return out;
}
function blocks(n, pageUrl, depth = 0) {
  const parts = [];
  let run = [];
  const flush = () => { const t = inline(run, pageUrl).trim(); if (t) parts.push(t); run = []; };
  for (const c of n.children) {
    if (c.text !== undefined || (!BLOCK.has(c.tag) && !skip(c))) { run.push(c); continue; }
    flush();
    if (skip(c)) continue;
    const t = c.tag;
    if (/^h[1-6]$/.test(t)) parts.push(`${"#".repeat(Number(t[1]))} ${inline(c.children, pageUrl).replace(/\[([^\]]*)\]\(([^)]*)\)/g, (m, text, href) => (href.startsWith(`${pageUrl}#`) ? text : m)).trim()}`);
    else if (t === "p") { const s = inline(c.children, pageUrl).trim(); if (s) parts.push(s); }
    else if (t === "pre") parts.push("```\n" + textOf(c).replace(/\n+$/, "") + "\n```");
    else if (t === "ul" || t === "ol") {
      const items = c.children.filter((x) => x.tag === "li");
      parts.push(items.map((li, i) => `${"  ".repeat(depth)}${t === "ol" ? `${i + 1}.` : "-"} ${blocks(li, pageUrl, depth + 1).replace(/\n\n/g, "\n").trim()}`).join("\n"));
    } else if (t === "table") {
      const rows = [];
      const walk = (x) => { if (x.tag === "tr") rows.push(x.children.filter((td) => td.tag === "td" || td.tag === "th").map((td) => inline(td.children, pageUrl).trim().replace(/\|/g, "\\|"))); else (x.children ?? []).forEach(walk); };
      walk(c);
      if (rows.length) parts.push([`| ${rows[0].join(" | ")} |`, `| ${rows[0].map(() => "---").join(" | ")} |`, ...rows.slice(1).map((r) => `| ${r.join(" | ")} |`)].join("\n"));
    } else if (/\sdoc-prompt\s/.test(cls(c))) parts.push(`> ${inline(c.children, pageUrl).trim()}`);
    else { const s = blocks(c, pageUrl, depth); if (s) parts.push(s); }
  }
  flush();
  return parts.join("\n\n");
}

// Returns the page as Markdown plus its h2 sections (with ids) for the search corpus.
function pageMarkdown(html, page) {
  const pageUrl = O + page.path;
  const main = find(parse(html), (n) => n.tag === "main");
  if (!main) throw new Error(`No <main> in rendered ${page.path}`);
  const body = blocks(main, pageUrl).replace(/\n{3,}/g, "\n\n").trim();
  const sections = [];
  const walk = (n) => {
    for (const c of n.children ?? []) {
      if (c.tag === "section" && c.attrs.id && /\sprose-sec\s/.test(cls(c))) {
        const h2 = find(c, (x) => x.tag === "h2");
        sections.push({ id: c.attrs.id, title: h2 ? textOf(h2).trim() : c.attrs.id, text: blocks(c, pageUrl).replace(/^## .*\n+/, "").trim() });
      } else if (c.tag) walk(c);
    }
  };
  walk(main);
  const front = `---\ntitle: ${JSON.stringify(page.title)}\ndescription: ${JSON.stringify(page.description)}\nurl: ${pageUrl}\n---\n\n`;
  return { markdown: front + body + "\n", sections };
}

// ---------- HTML shells ----------
let template = await readFile(join(dist, "index.html"), "utf8");
if (!template.includes('<div id="root"></div>')) throw new Error("dist/index.html has no empty #root to fill");

// Inline the main stylesheet to eliminate the render-blocking CSS request and preload primary Latin fonts
// so the browser fetches them in parallel with JS instead of chaining HTML -> CSS -> font.
const assetFiles = await readdir(join(dist, "assets"));
const fontPreloads = assetFiles
  .filter((f) => /^geist(?:-mono)?-latin-wght-normal-.*\.woff2$/.test(f))
  .sort()
  .map((f) => `    <link rel="preload" as="font" type="font/woff2" crossorigin href="/assets/${f}" />`)
  .join("\n");

const cssLinkRe = /<link rel="stylesheet"[^>]*href="(\/assets\/[^"]+\.css)"[^>]*>/;
const cssMatch = template.match(cssLinkRe);
if (cssMatch) {
  const css = await readFile(join(dist, cssMatch[1].slice(1)), "utf8");
  const safeCss = css.replace(/<\/style/gi, "<\\/style");
  template = template.replace(cssLinkRe, `${fontPreloads ? `${fontPreloads}\n    ` : ""}<style>${safeCss}</style>`);
} else if (fontPreloads) {
  template = template.replace("</head>", `${fontPreloads}\n  </head>`);
}

// Preload the current route's lazy JS chunk in parallel with the entry script to avoid an entry -> route chain.
const chunkByPrefix = (prefix) => assetFiles.find((f) => f.startsWith(`${prefix}-`) && f.endsWith(".js"));
const routeChunk = (path) => {
  if (path === "/") return chunkByPrefix("Home");
  if (path === "/docs") return chunkByPrefix("Docs");
  if (path === "/guides" || path.startsWith("/guides/")) return chunkByPrefix("Guides");
  if (path === "/about") return chunkByPrefix("About");
  if (path === "/contact") return chunkByPrefix("Contact");
  if (path === "/privacy") return chunkByPrefix("Privacy");
  if (path === "/terms") return chunkByPrefix("Terms");
  if (path === "/conditions") return chunkByPrefix("Conditions");
  return chunkByPrefix("NotFound");
};

function shell(page, html) {
  const url = page.path === "/" ? `${O}/` : `${O}${page.path}`;
  let out = template.replace('<div id="root"></div>', `<div id="root">${html}</div>`);
  const chunk = routeChunk(page.path);
  if (chunk) out = out.replace("</head>", `    <link rel="modulepreload" crossorigin href="/assets/${chunk}" />\n  </head>`);
  out = out.replace(/<title>[\s\S]*?<\/title>/, `<title>${esc(page.title)}</title>`);
  out = out.replace(/(<meta name="description" content=")[^"]*(")/, `$1${esc(page.description)}$2`);
  out = out.replace(/(<link rel="alternate" type="text\/markdown" href=")[^"]*(")/, `$1${page.markdown}$2`);
  if (page.notFound) {
    out = out.replace(/\s*<link rel="canonical"[^>]*>/, "").replace("<head>", '<head>\n    <meta name="robots" content="noindex" />');
  } else {
    out = out.replace(/(<link rel="canonical" href=")[^"]*(")/, `$1${url}$2`).replace(/(<meta property="og:url" content=")[^"]*(")/, `$1${url}$2`);
  }
  if (page.path !== "/") {
    out = out.replace(/(<meta (?:property="og:title"|name="twitter:title") content=")[^"]*(")/g, `$1${esc(page.title)}$2`);
    out = out.replace(/(<meta (?:property="og:description"|name="twitter:description") content=")[^"]*(")/g, `$1${esc(page.description)}$2`);
    const name = page.title.split(" · ")[0];
    const guide = page.path.startsWith("/guides/");
    const crumbs = [{ name: "airgap", item: `${O}/` }, ...(guide ? [{ name: "Guides", item: `${O}/guides` }] : []), { name, item: url }];
    const ld = {
      "@context": "https://schema.org",
      "@graph": [
        { "@type": "WebPage", name: page.title, description: page.description, url, isPartOf: { "@type": "WebSite", name: "airgap", url: `${O}/` }, about: { "@id": `${O}/#software` } },
        { "@type": "BreadcrumbList", itemListElement: crumbs.map((c, i) => ({ "@type": "ListItem", position: i + 1, ...c })) },
        ...(guide
          ? [{ "@type": "TechArticle", headline: name, description: page.description, url, inLanguage: "en", dateModified: today, author: { "@type": "Person", name: SITE.author.name, url: SITE.author.url }, publisher: { "@type": "Person", name: SITE.author.name }, about: { "@id": `${O}/#software` }, isPartOf: { "@type": "CollectionPage", name: "airgap guides", url: `${O}/guides` } }]
          : []),
      ],
    };
    out = out.replace(/<script type="application\/ld\+json">[\s\S]*?<\/script>/, `<script type="application/ld+json">${JSON.stringify(ld)}</script>`);
  }
  return out;
}

const chunks = [];
const addChunks = (url, title, markdown) => {
  // Split on ## / ### headings; keep each chunk small enough to return whole.
  const lines = markdown.replace(/^---[\s\S]*?---\n/, "").split("\n");
  let section = title;
  let buf = [];
  const push = () => {
    const text = buf.join("\n").trim();
    if (text.length > 40) chunks.push({ url, title, section, text: text.slice(0, 2400) });
    buf = [];
  };
  let fence = false;
  for (const line of lines) {
    if (line.startsWith("```")) fence = !fence;
    const h = !fence && /^(#{1,3})\s+(.*)/.exec(line);
    if (h) { push(); section = h[2].replace(/\*\*/g, "").trim(); continue; }
    buf.push(line);
  }
  push();
};

const pageMd = {};
for (const page of PAGES) {
  const html = render(page.path);
  const file = page.path === "/" ? "index.html" : `${page.path.slice(1)}/index.html`;
  await write(file, shell(page, html));
  const url = page.path === "/" ? `${O}/` : `${O}${page.path}`;
  if (page.path === "/") {
    pageMd[page.path] = await readFile(join(dist, "index.md"), "utf8");
    addChunks(url, "airgap", pageMd[page.path]);
  } else {
    const { markdown, sections } = pageMarkdown(html, page);
    pageMd[page.path] = markdown;
    await write(page.markdown.slice(1), markdown);
    for (const s of sections) if (s.text) chunks.push({ url: `${url}#${s.id}`, title: page.title, section: s.title, text: s.text.slice(0, 2400) });
  }
}
const nf = { path: "/404", title: "Page not found · airgap", description: "There is nothing at this address.", markdown: "/index.md", notFound: true };
await write("404.html", shell(nf, render("/404")));

// ---------- Repo docs mirrored onto this domain ----------
const docsDir = join(repo, "docs");
if (!(await exists(docsDir))) {
  throw new Error(`Cannot find ${docsDir}. On Vercel, enable "Include files outside the Root Directory in the Build Step".`);
}
const docs = [];
for (const name of (await readdir(docsDir)).filter((n) => n.endsWith(".md")).sort()) {
  const md = await readFile(join(docsDir, name), "utf8");
  await write(`docs/${name}`, md);
  const title = (/^#\s+(.*)/m.exec(md)?.[1] ?? name).trim();
  docs.push({ path: `/docs/${name}`, title, md });
  addChunks(`${O}/docs/${name}`, title, md);
}
if (await exists(join(docsDir, "assets"))) await cp(join(docsDir, "assets"), join(dist, "docs", "assets"), { recursive: true });
for (const name of ["README.md", "CHANGELOG.md", "SECURITY.md"]) {
  const md = await readFile(join(repo, name), "utf8");
  await write(name, md);
  docs.push({ path: `/${name}`, title: (/^#\s+(.*)/m.exec(md)?.[1] ?? name).trim(), md });
  if (name !== "README.md") addChunks(`${O}/${name}`, name.replace(".md", ""), md);
}

// ---------- Discovery documents, corpus, sitemap ----------
for (const [rel, body] of Object.entries(agentFiles())) await write(rel, JSON.stringify(body, null, 2) + "\n");
await write("agent/corpus.json", JSON.stringify({ generated: today, chunks }) + "\n");

const urls = [
  ...PAGES.map((p) => ({ loc: p.path === "/" ? `${O}/` : `${O}${p.path}`, priority: p.path === "/" ? "1.0" : "0.8" })),
  ...PAGES.map((p) => ({ loc: `${O}${p.markdown}`, priority: "0.5" })),
  ...docs.map((d) => ({ loc: `${O}${d.path}`, priority: "0.6" })),
  { loc: `${O}/llms.txt`, priority: "0.7" },
  { loc: `${O}/llms-full.txt`, priority: "0.5" },
];
await write(
  "sitemap.xml",
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls
    .map((u) => `  <url><loc>${u.loc}</loc><lastmod>${today}</lastmod><priority>${u.priority}</priority></url>`)
    .join("\n")}\n</urlset>\n`,
);

// ---------- llms-full.txt ----------
const llms = await readFile(join(dist, "llms.txt"), "utf8");
const strip = (md) => md.replace(/^---[\s\S]*?---\n+/, "");
const full = [
  llms.trim(),
  ...PAGES.map((p) => `\n\n---\n\nSource: ${p.path === "/" ? `${O}/` : O + p.path}\n\n${strip(pageMd[p.path]).trim()}`),
  ...docs.map((d) => `\n\n---\n\nSource: ${O}${d.path}\n\n${d.md.trim()}`),
].join("");
await write("llms-full.txt", full + "\n");

// ---------- Every same-site link in llms.txt must resolve to a built file ----------
// Paths answered by the Vercel functions (see vercel.json) have no file in dist.
const DYNAMIC = [/^\/api\//, /^\/api$/, /^\/ask$/, /^\/mcp$/, /^\/a2a$/, /^\/\.well-known\/mcp$/];
const broken = [];
for (const [, href] of llms.matchAll(/\]\(([^)\s]+)\)/g)) {
  const u = new URL(href, `${O}/`);
  if (u.origin !== O) continue;
  const p = decodeURIComponent(u.pathname);
  if (DYNAMIC.some((re) => re.test(p))) continue;
  const candidates = [join(dist, p), join(dist, p, "index.html")];
  if (!(await Promise.all(candidates.map(exists))).some(Boolean)) broken.push(href);
}
if (broken.length) throw new Error(`llms.txt links that would 404: ${broken.join(", ")}`);

console.log(`prerendered ${PAGES.length} pages + 404, ${docs.length} docs, ${chunks.length} search chunks, ${urls.length} sitemap URLs`);
