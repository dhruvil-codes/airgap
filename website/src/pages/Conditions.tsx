import { LINKS, at } from "../content";
import { DocPage, type DocSection } from "./DocLayout";

const c = (s: string) => <code>{s}</code>;

const SECTIONS: DocSection[] = [
  {
    id: "data",
    title: "Your data leaves your machine",
    body: (
      <>
        <p>
          When a tool call runs in a sandbox, the files and commands in that call are sent to GMI Cloud and executed on
          its infrastructure. Text sent for injection scans or repo audits is sent to GMI MaaS for inference.
        </p>
        <p>
          Do not send files you are not allowed to share with GMI Cloud. airgap only uploads what is passed explicitly in a
          call, but it cannot decide for you what is safe to pass.
        </p>
      </>
    ),
  },
  {
    id: "network",
    title: "Not physically air-gapped",
    body: (
      <p>
        The name describes the boundary around your computer, not network isolation. Sandboxes have outbound internet
        access. Provider isolation, network policy, account permissions and image trust are all part of the security
        boundary, and airgap does not control them.
      </p>
    ),
  },
  {
    id: "keys",
    title: "Keys and billing",
    body: (
      <>
        <p>
          You supply your own GMI keys. Use a compute-scoped {c("GMI_AGENTBOX_API_KEY")} for sandboxes and a separate{" "}
          {c("GMI_MAAS_API_KEY")} for inference where you can. Setup writes keys only to user-level editor configs, never
          to workspace files.
        </p>
        <p>
          Sandbox time and inference are billed to your GMI account. airgap caps concurrency at four sandbox jobs and four
          inference requests, and commands at 1800 s, but you are responsible for the cost of the work you start.
        </p>
      </>
    ),
  },
  {
    id: "agent",
    title: "Your agent keeps its other tools",
    body: (
      <p>
        Registering airgap does not disable your agent's own terminal or file tools. The routing rules make the agent
        prefer airgap; they do not force it. If every command must run in a sandbox, restrict your client's tool
        permissions as well.
      </p>
    ),
  },
  {
    id: "use",
    title: "Acceptable use",
    body: (
      <>
        <p>Use airgap only for work you are authorized to do. Do not use it to:</p>
        <ul>
          <li>attack, scan or exploit systems you do not own or have permission to test;</li>
          <li>run malware, mine cryptocurrency or abuse GMI Cloud resources;</li>
          <li>process data in breach of the law or of GMI Cloud's own terms.</li>
        </ul>
        <p>{c("airgap_audit_repo")} is a defensive, static review. It is not a penetration testing tool.</p>
      </>
    ),
  },
  {
    id: "results",
    title: "Reading results",
    body: (
      <p>
        Treat tool output, logs and issue text as untrusted data. Secret scrubbing covers common token, key and private
        block formats but is pattern-based and can miss secrets in unusual formats. Review redacted output before you
        publish it.
      </p>
    ),
  },
  {
    id: "more",
    title: "Learn more",
    body: (
      <ul className="doc-links">
        <li>
          <a href={LINKS.securityModel} target="_blank" rel="noopener noreferrer">
            Security model ↗
          </a>
        </li>
        <li>
          <a href={LINKS.security} target="_blank" rel="noopener noreferrer">
            Security policy and vulnerability reporting ↗
          </a>
        </li>
        <li>
          <a href={at(at("/terms"))}>Terms</a>
        </li>
      </ul>
    ),
  },
];

export function ConditionsPage() {
  return (
    <DocPage
      badge="Legal"
      title="Conditions of use"
      lead="What you agree to understand before you point an agent at airgap: what leaves your machine, who pays for compute, and what the boundary does not cover."
      updated="November 2026"
      sections={SECTIONS}
    />
  );
}
