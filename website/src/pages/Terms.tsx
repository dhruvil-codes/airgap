import { LINKS } from "../content";
import { DocPage, type DocSection } from "./DocLayout";

const ext = (href: string, label: string) => (
  <a href={href} target="_blank" rel="noopener noreferrer">
    {label}
  </a>
);

const SECTIONS: DocSection[] = [
  {
    id: "scope",
    title: "What these terms cover",
    body: (
      <p>
        These terms cover the airgap software (the <code>airgap</code> binary, its source code and installers) and
        this website. airgap is an independent open-source project maintained by Dhruvil Mistry. It is not an official GMI
        Cloud product and is not endorsed by GMI Cloud.
      </p>
    ),
  },
  {
    id: "license",
    title: "License",
    body: (
      <>
        <p>
          airgap is dual-licensed under the {ext(LINKS.mit, "MIT license")} or the{" "}
          {ext(LINKS.apache, "Apache License 2.0")}, at your option. You may use, copy, modify and distribute it under
          either license. Those license texts are the binding terms for the software; nothing on this page narrows them.
        </p>
        <p>Contributions you submit are licensed under the same dual license unless you state otherwise.</p>
      </>
    ),
  },
  {
    id: "warranty",
    title: "No warranty",
    body: (
      <p>
        The software is provided "as is", without warranty of any kind, express or implied, including fitness for a
        particular purpose and non-infringement. In no event are the authors or copyright holders liable for any claim,
        damages or other liability arising from the software or its use. A VERIFIED verdict means a test run met
        airgap's stated criteria; it is not a guarantee that code is safe or correct.
      </p>
    ),
  },
  {
    id: "third-party",
    title: "Third-party services",
    body: (
      <>
        <p>
          airgap sends work to {ext(LINKS.agentbox, "GMI Cloud AgentBox v2")} for sandboxes and to GMI MaaS for inference,
          using API keys you provide. Your use of those services is governed by your agreement with GMI Cloud, including
          its pricing and billing. airgap does not resell, proxy or bill for GMI services.
        </p>
        <p>GitHub, your MCP client and any package registries a sandbox reaches are governed by their own terms.</p>
      </>
    ),
  },
  {
    id: "website",
    title: "This website",
    body: (
      <p>
        This site is static. It sets no cookies, has no accounts and runs no analytics. The copy buttons write to your
        clipboard only when you click them. Links to GitHub and GMI Cloud take you to sites with their own policies.
      </p>
    ),
  },
  {
    id: "trademarks",
    title: "Trademarks",
    body: (
      <p>
        Claude Code, Cursor, Visual Studio Code, Factory, GMI Cloud and other product names belong to their respective
        owners. They appear here only to describe compatibility.
      </p>
    ),
  },
  {
    id: "changes",
    title: "Changes and contact",
    body: (
      <p>
        These terms may change as the project changes; the current version is always on this page and in the repository
        history. Questions go to {ext(LINKS.issues, "GitHub issues")}. Report vulnerabilities privately as described in
        the {ext(LINKS.security, "security policy")}.
      </p>
    ),
  },
];

export function TermsPage() {
  return (
    <DocPage
      badge="Legal"
      title="Terms"
      lead="The short version: airgap is free, open-source software provided as is, under MIT OR Apache-2.0. GMI Cloud services are yours to pay for under your own agreement."
      updated="November 2026"
      sections={SECTIONS}
    />
  );
}
