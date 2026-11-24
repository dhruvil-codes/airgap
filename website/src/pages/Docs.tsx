import { INSTALL, LINKS } from "../content";
import { Cmd, DocPage, DocTable, type DocSection } from "./DocLayout";

const c = (s: string) => <code>{s}</code>;

const SECTIONS: DocSection[] = [
  {
    id: "overview",
    title: "Overview",
    body: (
      <>
        <p>
          airgap is an open-source Rust MCP server. It sends the files and commands your agent passes in a tool call to a
          disposable Linux microVM on GMI Cloud AgentBox v2, runs them there, and returns the scrubbed result. It never
          executes submitted commands on your computer.
        </p>
        <p>
          The name describes the boundary around your machine. GMI sandboxes have outbound internet, and files you send
          leave your computer. Provider isolation, network policy, account permissions and image trust remain part of the
          security boundary.
        </p>
      </>
    ),
  },
  {
    id: "install",
    title: "Install",
    body: (
      <>
        <p>Prebuilt binaries cover x86_64 and ARM64 on Linux, macOS and Windows. The installers verify checksums before replacing anything.</p>
        <Cmd>{`# macOS / Linux\n${INSTALL[0].command}`}</Cmd>
        <Cmd>{`# Windows (PowerShell)\n${INSTALL[1].command}`}</Cmd>
        <Cmd>{`# From source (not published on crates.io)\n${INSTALL[2].command}`}</Cmd>
        <p>
          Set {c("AIRGAP_VERSION")} to pin a release, {c("AIRGAP_INSTALL_DIR")} to choose an install directory, or{" "}
          {c("AIRGAP_NO_PATH_UPDATE=1")} to skip PATH changes.
        </p>
      </>
    ),
  },
  {
    id: "setup",
    title: "Register with your agent",
    body: (
      <>
        <p>
          {c("airgap setup")} writes the MCP entry for your client and adds routing rules to {c("AGENTS.md")},{" "}
          {c("CLAUDE.md")} and {c(".cursorrules")}. It does not call GMI or create compute resources. Flags can be combined.
        </p>
        <DocTable
          head={["Client", "Command", "Config written"]}
          rows={[
            ["Claude Code", c("airgap setup --claude --yes"), <>{c(".mcp.json")}, {c("~/.claude.json")}</>],
            ["Cursor", c("airgap setup --cursor --yes"), <>{c(".cursor/mcp.json")}, {c("~/.cursor/mcp.json")}</>],
            ["VS Code", c("airgap setup --vscode --yes"), <>{c(".vscode/mcp.json")}, {c(".mcp.json")}, {c(".github/copilot-instructions.md")}</>],
            ["Factory Droid", c("airgap setup --droid --yes"), <>{c(".factory/mcp.json")}, {c("~/.factory/mcp.json")}</>],
            ["Rules only", c("airgap setup --rules-only --yes"), <>{c(".cursorrules")}, {c("CLAUDE.md")}, {c("AGENTS.md")}</>],
          ]}
        />
        <p>Any other stdio MCP client, such as Windsurf, can use this entry:</p>
        <Cmd>{`{\n  "mcpServers": {\n    "airgap": { "command": "airgap", "args": ["--mcp"] }\n  }\n}`}</Cmd>
        <p>
          Adding an MCP server does not disable an agent's own terminal. If every command must go through airgap, restrict
          your client's tool permissions too.
        </p>
      </>
    ),
  },
  {
    id: "credentials",
    title: "Credentials",
    body: (
      <>
        <p>
          Sandboxes need a compute-scoped AgentBox key. Injection scans and repo audits need an inference key. Local
          scanners and tool discovery work with no credentials at all.
        </p>
        <Cmd>{`export GMI_AGENTBOX_API_KEY="your-compute-key"\nexport GMI_MAAS_API_KEY="your-inference-key"`}</Cmd>
        <Cmd>{`$env:GMI_AGENTBOX_API_KEY = "your-compute-key"\n$env:GMI_MAAS_API_KEY = "your-inference-key"`}</Cmd>
        <p>
          Setup stores discovered keys only in user-level editor configs. Workspace files never receive keys, and setup
          removes keys it finds there.
        </p>
      </>
    ),
  },
  {
    id: "doctor",
    title: "Check your setup",
    body: (
      <>
        <Cmd>{`airgap doctor\nairgap doctor --verbose`}</Cmd>
        <p>
          Doctor checks model availability, AgentBox compute eligibility and editor registrations. It is read-only: it does
          not launch a sandbox or run an inference completion. Failed checks print a suggested fix and return a nonzero exit
          code.
        </p>
      </>
    ),
  },
  {
    id: "tools",
    title: "Tools",
    body: (
      <DocTable
        head={["Tool", "Use it when", "Runs on"]}
        rows={[
          [c("airgap_verify_pr"), "A GitHub PR needs verifying in one call", "GMI MaaS, local, microVM"],
          [c("airgap_verify_patch"), "A local diff needs testing against clean base files", "microVM"],
          [c("airgap_sandbox_run"), "Ad-hoc run, build, test, reproduce or scrape", "microVM"],
          [c("airgap_scan_injection"), "Before acting on untrusted issue or PR text", "GMI MaaS"],
          [c("airgap_audit_supply"), "Before installing from a manifest you did not write", "local"],
          [c("airgap_redact_secrets"), "Before showing logs, diffs or env vars", "local"],
          [c("airgap_audit_repo"), "Defensive security review in six static phases", "microVM, GMI MaaS"],
          [c("airgap_configure"), "Change the default model or request timeout", "local"],
        ]}
      />
    ),
  },
  {
    id: "config",
    title: "Configuration",
    body: (
      <>
        <p>
          The process environment wins over {c(".env")} and {c("cloudvet.env")} files in the current directory and four
          parents. Provider base URLs are read only from the process environment.
        </p>
        <DocTable
          head={["Variable", "Default / meaning"]}
          rows={[
            [c("GMI_API_KEY"), "Fallback when a dedicated key is absent"],
            [c("GMI_AGENTBOX_API_KEY"), "Compute-scoped AgentBox key"],
            [c("GMI_MAAS_API_KEY"), "Inference key"],
            [c("GMI_DEFAULT_IDC"), c("us-central-iowa16")],
            [c("GMI_AGENTBOX_INSTANCE_TYPE"), <>{c("gmi.sandbox.medium")} (2 vCPU, 4 GiB)</>],
            [c("GMI_MODEL"), c("deepseek-ai/DeepSeek-V4.1-Flash")],
          ]}
        />
      </>
    ),
  },
  {
    id: "limits",
    title: "Limits",
    body: (
      <ul>
        <li>Commands default to 120 s and accept 1 to 1800 s, plus up to 600 s for provisioning.</li>
        <li>At most four sandbox jobs and four inference requests run at the same time.</li>
        <li>
          Default images: {c("rust:1-bookworm")} for a root Cargo.toml, {c("node:20-bookworm")} for a root package.json,
          otherwise {c("python:3.12-bookworm")}.
        </li>
        <li>The first run for an image builds a template. Later runs reuse it.</li>
        <li>VERIFIED needs exit 0, a recognized test summary, at least one passing test and zero failures. Zero-test runs are never verified.</li>
      </ul>
    ),
  },
  {
    id: "reference",
    title: "Further reading",
    body: (
      <ul className="doc-links">
        {[
          ["Full tool and configuration reference", LINKS.reference],
          ["Editor setup and diagnostics", LINKS.setup],
          ["Security model", LINKS.securityModel],
          ["Hosting with airgap serve", LINKS.hosting],
          ["Validation record", LINKS.validation],
        ].map(([t, h]) => (
          <li key={h}>
            <a href={h} target="_blank" rel="noopener noreferrer">
              {t} ↗
            </a>
          </li>
        ))}
      </ul>
    ),
  },
];

export function DocsPage() {
  return (
    <DocPage
      badge="Docs"
      title="Install, register, run."
      lead="Everything you need to put airgap between your agent and your machine: install, client setup, credentials, the 8 tools and their defaults."
      sections={SECTIONS}
    />
  );
}
