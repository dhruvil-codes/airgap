import { SITE } from "../agent/knowledge";
import { LINKS, at } from "../content";
import { DocPage, type DocSection } from "./DocLayout";

const ext = (href: string, label: string) => (
  <a href={href} target="_blank" rel="noopener noreferrer">
    {label}
  </a>
);
const c = (s: string) => <code>{s}</code>;

const SECTIONS: DocSection[] = [
  {
    id: "website",
    title: "This website",
    body: (
      <>
        <p>
          This site sets no cookies, has no accounts, and runs no analytics, tracking pixels or third-party scripts. Fonts
          are served from this domain. The copy buttons write to your clipboard only when you click them.
        </p>
        <p>
          The site is hosted on Vercel, which processes standard request data (such as IP address, user agent and the URL
          requested) to serve pages and keep the service secure, as described in Vercel's own privacy policy.
        </p>
      </>
    ),
  },
  {
    id: "agent-endpoints",
    title: "Agent endpoints on this site",
    body: (
      <p>
        The read-only endpoints for AI agents ({c("/ask")}, {c("/mcp")}, {c("/a2a")} and {c("/api/v1")}) use the question
        text you send only to build the answer. The site's code does not store it, does not send it to any model or third
        party, and does not tie it to an identity. Vercel's request logs still apply.
      </p>
    ),
  },
  {
    id: "software",
    title: "The airgap software",
    body: (
      <>
        <p>
          The {c("airgap")} binary has no telemetry and does not contact this website. It talks only to the providers you
          configure: GMI Cloud AgentBox for sandboxes, GMI MaaS for inference, and GitHub when you verify a pull request.
        </p>
        <p>
          Only what you or your agent pass in a tool call leaves your machine: the files, the command, and text sent for
          injection scans or audits. That data is processed by GMI Cloud under your agreement with them. Setup stores
          discovered keys only in user-level editor configs, never in workspace files.
        </p>
      </>
    ),
  },
  {
    id: "contact",
    title: "Questions",
    body: (
      <p>
        Email {<a href={`mailto:${SITE.contactEmail}`}>{SITE.contactEmail}</a>} or see the{" "}
        <a href={at("/contact")}>contact page</a>. For how data moves through a tool call, read the{" "}
        {ext(LINKS.securityModel, "security model")}.
      </p>
    ),
  },
];

export function PrivacyPage() {
  return (
    <DocPage
      badge="Legal"
      title="Privacy"
      lead="The short version: this site collects nothing about you, and the airgap binary sends only what you pass in a tool call."
      updated="November 2026"
      sections={SECTIONS}
    />
  );
}
