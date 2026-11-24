import { AGENT_ENDPOINTS, SITE } from "../agent/knowledge";
import { LINKS, at } from "../content";
import { DocPage, DocTable, type DocSection } from "./DocLayout";

const ext = (href: string, label: string) => (
  <a href={href} target="_blank" rel="noopener noreferrer">
    {label}
  </a>
);
const c = (s: string) => <code>{s}</code>;

const SECTIONS: DocSection[] = [
  {
    id: "what",
    title: "What airgap is",
    body: (
      <>
        <p>{SITE.summary}</p>
        <p>
          It exists because coding agents now install packages, run builds and execute tests on their own. Those commands run
          with your user's permissions, next to your SSH keys, cloud credentials and every repository you have checked out.
          airgap moves that work into a remote, single-use microVM and returns only the scrubbed result.
        </p>
      </>
    ),
  },
  {
    id: "who",
    title: "Who builds it",
    body: (
      <>
        <p>
          airgap is written and maintained by {ext(SITE.author.url, SITE.author.name)}, an independent developer. It is not
          an official GMI Cloud product and is not endorsed by GMI Cloud. It uses {ext(LINKS.agentbox, "GMI Cloud AgentBox v2")}{" "}
          for sandboxes and GMI MaaS for inference, with API keys you provide.
        </p>
        <p>
          The project is developed in the open on {ext(LINKS.repo, "GitHub")}. Contributions follow the{" "}
          {ext(LINKS.contributing, "contributing guide")} and the {ext(LINKS.conduct, "code of conduct")}.
        </p>
      </>
    ),
  },
  {
    id: "facts",
    title: "Project facts",
    body: (
      <DocTable
        head={["Fact", "Value"]}
        rows={[
          ["Current version", c(`v${SITE.version}`)],
          ["License", <>MIT OR Apache-2.0 ({ext(LINKS.mit, "MIT")}, {ext(LINKS.apache, "Apache-2.0")})</>],
          ["Language", "Rust (one static binary, no Docker or Python needed locally)"],
          ["Platforms", "x86_64 and ARM64 on Linux, macOS and Windows"],
          ["Protocol", <>MCP over stdio ({c("airgap --mcp")}) or Streamable HTTP ({c("airgap serve")})</>],
          ["MCP Registry", ext(SITE.registryUrl, SITE.registryName)],
          ["npm", <>{ext(SITE.npmUrl, SITE.npmPackage)} (run {c("npx -y airgap-mcp")})</>],
          ["Price", "Free. GMI Cloud bills sandbox time and inference to your own account."],
        ]}
      />
    ),
  },
  {
    id: "agents",
    title: "For AI agents",
    body: (
      <>
        <p>
          Everything on this site is also available in machine-readable form. Nothing here needs an account or an API key,
          and every endpoint is read-only: it answers questions about airgap and never runs code.
        </p>
        <DocTable
          head={["Path", "What it is"]}
          rows={AGENT_ENDPOINTS.map((e) => [<a href={at(e.path)}>{c(e.path)}</a>, e.purpose])}
        />
        <p>
          To run code in a sandbox, install the {c("airgap")} binary and register it with your MCP client. The hosted{" "}
          {c("/mcp")} endpoint on this site only serves documentation.
        </p>
      </>
    ),
  },
  {
    id: "elsewhere",
    title: "Find airgap elsewhere",
    body: (
      <ul className="doc-links">
        <li>{ext(LINKS.repo, "Source code on GitHub ↗")}</li>
        <li>{ext(LINKS.releases, "Releases and prebuilt binaries ↗")}</li>
        <li>{ext(SITE.registryUrl, "Official MCP Registry entry ↗")}</li>
        <li>{ext(SITE.npmUrl, "npm package ↗")}</li>
        <li>{ext(LINKS.changelog, "Changelog ↗")}</li>
        <li>
          <a href={at("/contact")}>Contact the maintainer</a>
        </li>
      </ul>
    ),
  },
];

export function AboutPage() {
  return (
    <DocPage
      badge="About"
      title="About airgap"
      lead="An independent, open-source MCP server that gives AI coding agents a disposable Linux machine to run on, so your own machine stays out of reach."
      sections={SECTIONS}
    />
  );
}
