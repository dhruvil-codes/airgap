// Plain data (no JSX) so the Vite config, agent knowledge and prerenderer can import it.
export type GuideGroup = "walkthroughs" | "security";

export type GuideMeta = {
  slug: string;
  group: GuideGroup;
  title: string;
  card: string;
  summary: string;
};

export const GUIDE_GROUPS: { id: GuideGroup; title: string; lead: string }[] = [
  { id: "walkthroughs", title: "Walkthroughs", lead: "The jobs airgap does every day. Ask in plain words; the routing rules pick the tool." },
  { id: "security", title: "Security deep dives", lead: "Why the boundary is built the way it is, and how to harden a team around it." },
];

export const GUIDE_META: GuideMeta[] = [
  {
    slug: "verify-pr",
    group: "walkthroughs",
    title: "Verify a pull request",
    card: "Scan, clone at the pinned SHA and run the tests in one call.",
    summary: "Verify a GitHub pull request with one airgap_verify_pr call: injection scan, manifest audit, SHA-pinned clone, build and tests in a disposable microVM.",
  },
  {
    slug: "verify-patch",
    group: "walkthroughs",
    title: "Test a local patch",
    card: "Send clean base files and a diff. Your working tree stays put.",
    summary: "Test an uncommitted patch with airgap_verify_patch: the diff is applied to clean base files inside a microVM and the tests run there.",
  },
  {
    slug: "run",
    group: "walkthroughs",
    title: "Run tests or a one-off command",
    card: "The image is picked from your files. The VM is deleted after.",
    summary: "Run tests, builds or one-off commands in a fresh Linux microVM with airgap_sandbox_run. Node, Python and Rust images are picked from your files.",
  },
  {
    slug: "issue-text",
    group: "walkthroughs",
    title: "Act on untrusted issue text",
    card: "Scan issue bodies and comments before the agent acts on them.",
    summary: "Scan GitHub issue text for hidden prompt-injection instructions with airgap_scan_injection before your coding agent acts on it.",
  },
  {
    slug: "manifest",
    group: "walkthroughs",
    title: "Audit a dependency manifest",
    card: "Catch install hooks and risky packages before anything installs.",
    summary: "Audit package.json, Cargo.toml or requirements.txt for install hooks, git URLs and risky packages with airgap_audit_supply, locally and with no keys.",
  },
  {
    slug: "scrub",
    group: "walkthroughs",
    title: "Scrub logs before sharing",
    card: "Replace keys, tokens and private blocks with [REDACTED].",
    summary: "Redact API keys, bearer tokens and private key blocks from logs, diffs and environment dumps with airgap_redact_secrets before you share them.",
  },
  {
    slug: "audit",
    group: "security",
    title: "Run a security review",
    card: "A defensive, static review of a repository in six phases.",
    summary: "Run a defensive, static six-phase security review of a repository with airgap_audit_repo, using a microVM and GMI MaaS.",
  },
  {
    slug: "docker-vs-microvm",
    group: "security",
    title: "Docker vs. remote microVMs",
    card: "Why a local container is not a security boundary for agents.",
    summary: "Why local Docker containers are not a security boundary for AI coding agents, and how disposable remote microVMs with their own kernel differ.",
  },
  {
    slug: "prompt-injection-rce",
    group: "security",
    title: "Stopping prompt injection RCE",
    card: "Break the lethal trifecta before untrusted text can run.",
    summary: "How prompt injection turns into remote code execution on a developer machine, and how airgap breaks the lethal trifecta before execution starts.",
  },
  {
    slug: "supply-chain-hardening",
    group: "security",
    title: "Blocking malicious install scripts",
    card: "postinstall, setup.py and build.rs hooks are refused by default.",
    summary: "Defend against malicious npm postinstall, setup.py and build.rs hooks: airgap audits manifests locally and refuses lifecycle scripts by default.",
  },
  {
    slug: "secret-scrubbing",
    group: "security",
    title: "Zero credential leakage",
    card: "Known key formats plus entropy checks, run on your machine.",
    summary: "Keep credentials out of your agent's context window: airgap redacts known key formats and high-entropy tokens locally before output is returned.",
  },
  {
    slug: "team-playbook",
    group: "security",
    title: "The zero-trust MCP playbook",
    card: "Routing rules plus client permissions for a whole team.",
    summary: "Roll airgap out across a team on Claude Code, Cursor, VS Code and Factory Droid with managed routing rules and client permission gates.",
  },
];

export const guidePath = (slug: string) => `/guides/${slug}`;
