import type { LucideIcon } from "lucide-react";
import { BookOpen, Compass, FileText, Layers, Scale, ShieldCheck, Terminal, Workflow } from "lucide-react";

// Vite base URL without the trailing slash, so links keep working if the site moves under a sub-path.
export const BASE = import.meta.env.BASE_URL.replace(/\/$/, "");
export const at = (path: string) => BASE + path;

export const VERSION = "0.2.0";
export const REPO = "https://github.com/dhruvil-codes/airgap";
export const RAW = "https://raw.githubusercontent.com/dhruvil-codes/airgap/main/scripts";

export const LINKS = {
  repo: REPO,
  releases: `${REPO}/releases`,
  issues: `${REPO}/issues`,
  changelog: `${REPO}/blob/main/CHANGELOG.md`,
  contributing: `${REPO}/blob/main/CONTRIBUTING.md`,
  conduct: `${REPO}/blob/main/CODE_OF_CONDUCT.md`,
  security: `${REPO}/blob/main/SECURITY.md`,
  docs: `${REPO}/tree/main/docs`,
  setup: `${REPO}/blob/main/docs/setup.md`,
  hosting: `${REPO}/blob/main/docs/hosting.md`,
  reference: `${REPO}/blob/main/docs/reference.md`,
  validation: `${REPO}/blob/main/docs/validation.md`,
  agentbox: "https://docs.gmicloud.ai/agentbox-marketplace/overview",
  gmi: "https://docs.gmicloud.ai/",
  author: "https://github.com/dhruvil-codes",
  mit: `${REPO}/blob/main/LICENSE-MIT`,
  apache: `${REPO}/blob/main/LICENSE-APACHE`,
  securityModel: `${REPO}/blob/main/docs/security-model.md`,
  mcpRegistry: "https://registry.modelcontextprotocol.io/v0/servers?search=dev.useairgap/airgap",
} as const;

export type NavLink = { href: string; title: string; desc: string; icon: LucideIcon; external?: boolean };

export const PRODUCT_NAV: NavLink[] = [
  { href: at("/#boundary"), title: "How it works", desc: "Follow one call across the microVM boundary", icon: Workflow },
  { href: at("/#problem"), title: "The problem & fix", desc: "Why host execution breaks and how airgap isolates it", icon: Layers },
  { href: at("/#gates"), title: "Security gates", desc: "Injection scan, supply audit, isolation and secret scrub", icon: ShieldCheck },
  { href: at("/#setup"), title: "Quick setup", desc: "One command per client, then ask in plain words", icon: Terminal },
];

export const RESOURCE_NAV: NavLink[] = [
  { href: at("/docs"), title: "Docs", desc: "Install, setup, credentials, all 8 tools and config", icon: BookOpen },
  { href: at("/guides"), title: "Guides", desc: "Step-by-step: verify a PR, test a patch, scrub logs", icon: Compass },
  { href: LINKS.changelog, title: "Changelog", desc: "Everything that shipped in v0.2.0", icon: FileText, external: true },
  { href: at("/terms"), title: "Terms & conditions", desc: "License, warranty, data and usage conditions", icon: Scale },
];

export const INSTALL = [
  { value: "sh", label: "macOS / Linux", prompt: "$", command: `curl -fsSL ${RAW}/install.sh | bash` },
  { value: "ps", label: "Windows", prompt: ">", command: `irm ${RAW}/install.ps1 | iex` },
  { value: "cargo", label: "Cargo", prompt: "$", command: `cargo install --git ${REPO} --locked` },
] as const;

export const CLIENTS = [
  { name: "Claude Code", logo: at("/logos/claude-code.svg"), tab: "claude" },
  { name: "Cursor", logo: at("/logos/cursor.svg"), tab: "cursor" },
  { name: "VS Code", logo: at("/logos/vscode.svg"), tab: "vscode" },
  { name: "Factory Droid", logo: at("/logos/factory.svg"), tab: "droid" },
  { name: "Any stdio MCP client", logo: at("/logos/mcp.svg"), tab: "mcp" },
] as const;

export const ISOLATION_POINTS = [
  { text: "A fresh Linux microVM for every run, deleted afterwards", on: true },
  { text: "Only the files you pass in the call leave your machine", on: true },
  { text: "Same Linux environment from Windows, macOS or Linux", on: true },
  { text: "Templates cache after the first run", on: false },
] as const;

export type Tone = "cmt" | "prompt" | "flag" | "str" | "key" | "ok" | "bad" | "dim";
export type Seg = string | readonly [Tone, string];
export type CodeLine = readonly Seg[];

export type SetupClient = {
  value: string;
  name: string;
  logo: string;
  invert?: boolean;
  flag?: string;
  json?: string;
  writes: string;
  prompt: string;
};

export const MCP_JSON = `{
  "mcpServers": {
    "airgap": { "command": "airgap", "args": ["--mcp"] }
  }
}`;

export const SETUP_CLIENTS: SetupClient[] = [
  { value: "claude", name: "Claude Code", logo: at("/logos/claude-code.svg"), flag: "--claude", writes: ".mcp.json and CLAUDE.md", prompt: "use airgap to verify acme/api#142" },
  { value: "cursor", name: "Cursor", logo: at("/logos/cursor.svg"), invert: true, flag: "--cursor", writes: ".cursor/mcp.json and .cursorrules", prompt: "use airgap and run the tests" },
  { value: "vscode", name: "VS Code", logo: at("/logos/vscode.svg"), flag: "--vscode", writes: ".vscode/mcp.json and .github/copilot-instructions.md", prompt: "verify this patch with airgap" },
  { value: "droid", name: "Factory Droid", logo: at("/logos/factory.svg"), flag: "--droid", writes: ".factory/mcp.json and AGENTS.md", prompt: "use airgap and run the tests" },
  { value: "mcp", name: "Any MCP client", logo: at("/logos/mcp.svg"), invert: true, json: MCP_JSON, writes: "Paste this into your client's MCP config. Works with Windsurf and any other stdio client.", prompt: "use airgap to verify acme/api#142" },
];
export const FAQ: { q: string; a: string }[] = [
  {
    q: "Is it really air-gapped?",
    a: "No. The name describes the boundary around your machine. GMI sandboxes have outbound internet, and files you send leave your computer. Provider isolation, network policy, account permissions and image trust are still part of the security boundary.",
  },
  {
    q: "What leaves my machine?",
    a: "Only what you or your agent pass explicitly: the files in a tool call, the command, and the text sent for injection scans or audits. airgap never uploads files implicitly and never reads your working tree for patch verification.",
  },
  {
    q: "Does it stop my agent from using its own terminal?",
    a: "No. Adding an MCP server does not disable an agent's other tools. airgap setup writes routing rules into AGENTS.md, CLAUDE.md and .cursorrules so the agent prefers airgap. If all execution must go through it, restrict your client's tool permissions too.",
  },
  {
    q: "What do I need to run it?",
    a: "The airgap binary and a GMI Cloud account. Sandboxes need a compute-scoped AgentBox key; injection scans and repo audits need an inference key. Local scanners and tool discovery work with no credentials at all.",
  },
  {
    q: "Which agents does it work with?",
    a: "airgap setup configures Claude Code, Cursor, VS Code and Factory Droid. Any other stdio MCP client, Windsurf included, can use the JSON config. airgap serve exposes the same tools over HTTP for hosted or shared setups.",
  },
  {
    q: "Is this an official GMI Cloud product?",
    a: "No. airgap is an independent open-source project by Dhruvil Mistry that uses GMI Cloud AgentBox v2 for sandboxes and GMI MaaS for inference.",
  },
  {
    q: "How is airgap different from running Docker on my laptop?",
    a: "Local Docker shares your host kernel, mounts local sockets or volumes, requires a heavy background daemon (Docker Desktop), and still risks host contamination if a container escapes. airgap executes commands in single-use disposable Linux microVMs on GMI Cloud—zero local Docker required, zero host resource overhead, and the sandbox is destroyed immediately when done.",
  },
  {
    q: "Does airgap require Docker or local virtual machines installed?",
    a: "No. airgap requires zero local Docker daemons, zero local VMs, and zero local container runtimes. It is a single, zero-dependency native Rust binary (or runnable directly via 'npx -y airgap-mcp').",
  },
  {
    q: "Can I use airgap without installing Rust or Cargo?",
    a: "Yes. If you have Node.js 18+, run 'npx -y airgap-mcp setup --yes'. You can also download prebuilt standalone binaries for Linux, macOS, and Windows via our one-line shell and PowerShell installers.",
  },
];

export const FOOTER_COLUMNS: { heading: string; links: { label: string; href: string; external?: boolean }[] }[] = [
  {
    heading: "Product",
    links: [
      { label: "How it works", href: at("/#boundary") },
      { label: "Security gates", href: at("/#gates") },
      { label: "Setup", href: at("/#setup") },
      { label: "FAQ", href: at("/#faq") },
    ],
  },
  {
    heading: "Learn",
    links: [
      { label: "Docs", href: at("/docs") },
      { label: "Guides", href: at("/guides") },
      { label: "Changelog", href: LINKS.changelog, external: true },
      { label: "Releases", href: LINKS.releases, external: true },
      { label: "llms.txt", href: at("/llms.txt") },
    ],
  },
  {
    heading: "Legal",
    links: [
      { label: "Terms", href: at("/terms") },
      { label: "Conditions", href: at("/conditions") },
      { label: "Privacy", href: at("/privacy") },
      { label: "MIT license", href: LINKS.mit, external: true },
      { label: "Apache-2.0 license", href: LINKS.apache, external: true },
    ],
  },
  {
    heading: "Community",
    links: [
      { label: "About", href: at("/about") },
      { label: "Contact", href: at("/contact") },
      { label: "GitHub", href: LINKS.repo, external: true },
      { label: "Issues", href: LINKS.issues, external: true },
      { label: "Security policy", href: LINKS.security, external: true },
    ],
  },
];
