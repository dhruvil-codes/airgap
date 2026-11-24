import { SITE } from "../agent/knowledge";
import { LINKS, at } from "../content";
import { DocPage, type DocSection } from "./DocLayout";

const ext = (href: string, label: string) => (
  <a href={href} target="_blank" rel="noopener noreferrer">
    {label}
  </a>
);
const mail = (subject: string) => (
  <a href={`mailto:${SITE.contactEmail}?subject=${encodeURIComponent(subject)}`}>{SITE.contactEmail}</a>
);

const SECTIONS: DocSection[] = [
  {
    id: "questions",
    title: "Questions, bugs and ideas",
    body: (
      <>
        <p>
          Open a {ext(LINKS.issues, "GitHub issue")}. The templates ask for the airgap version, your OS and MCP client, and
          the command or tool call that failed. Run {<code>airgap doctor --verbose</code>} first and include its output;
          it never prints key values.
        </p>
        <p>Do not paste API keys, sandbox tokens or private logs into a public issue. Scrub them with {<code>airgap_redact_secrets</code>} first.</p>
      </>
    ),
  },
  {
    id: "security",
    title: "Security reports",
    body: (
      <p>
        Report vulnerabilities privately by email to {mail("airgap security report")} with the subject{" "}
        <code>airgap security report</code>. The {ext(LINKS.security, "security policy")} explains what to include and what is
        in scope.
      </p>
    ),
  },
  {
    id: "other",
    title: "Everything else",
    body: (
      <p>
        For anything that is not a bug or a security report, email {mail("airgap")}. Replies are best effort; there is no
        paid support contract. The maintainer is {ext(SITE.author.url, SITE.author.name)} on GitHub.
      </p>
    ),
  },
  {
    id: "gmi",
    title: "GMI Cloud accounts and billing",
    body: (
      <p>
        airgap does not manage GMI Cloud accounts, keys or invoices. Contact {ext(LINKS.gmi, "GMI Cloud")} directly for
        those. See the <a href={at("/about")}>about page</a> for how the two relate.
      </p>
    ),
  },
];

export function ContactPage() {
  return (
    <DocPage
      badge="Contact"
      title="Contact"
      lead="GitHub issues for bugs and questions, private email for security reports. airgap is maintained by one person, so replies are best effort."
      sections={SECTIONS}
    />
  );
}
