import { LINKS } from "../content";
import { Cmd, DocPage, DocTable, Prompt, type DocSection } from "./DocLayout";

const c = (s: string) => <code>{s}</code>;

const SECTIONS: DocSection[] = [
  {
    id: "before",
    title: "Before you start",
    body: (
      <>
        <p>
          Every guide assumes airgap is installed and registered with your client. If {c("airgap doctor")} prints no
          failures, you are ready.
        </p>
        <Cmd>{`airgap setup --claude --yes\nairgap doctor`}</Cmd>
        <p>
          You talk to your agent in plain words. The routing rules that setup writes tell it which airgap tool to call, so
          you never have to name one.
        </p>
      </>
    ),
  },
  {
    id: "verify-pr",
    title: "Verify a pull request",
    body: (
      <>
        <p>Give your agent the repository and PR number, or the full PR URL.</p>
        <Prompt>use airgap to verify acme/api#142</Prompt>
        <p>The agent calls {c("airgap_verify_pr")} once. That single call:</p>
        <ol>
          <li>Scans the PR title, body and diff for prompt injection before any compute starts.</li>
          <li>Audits changed manifests for install hooks, git URLs and risky packages.</li>
          <li>Clones the PR head at its pinned commit SHA inside a fresh microVM.</li>
          <li>Installs, builds and runs the test suite, then deletes the VM.</li>
          <li>Returns a verdict with secrets in the logs replaced by {c("[REDACTED]")}.</li>
        </ol>
        <p>
          A PR is VERIFIED only when the run exits 0, a test summary is recognized, at least one test passes and none fail.
          A run with zero tests is never verified.
        </p>
      </>
    ),
  },
  {
    id: "verify-patch",
    title: "Test a local patch",
    body: (
      <>
        <p>
          For changes that are not on GitHub yet, ask the agent to verify the diff. It sends the clean base files and the
          unified diff; airgap applies the patch inside the microVM and runs the tests there.
        </p>
        <Prompt>use airgap to verify this patch before I commit it</Prompt>
        <p>
          airgap never reads your working tree on its own. Only the files the agent passes in the call leave your machine.
        </p>
      </>
    ),
  },
  {
    id: "run",
    title: "Run tests or a one-off command",
    body: (
      <>
        <p>
          For anything else (a test run, a build, reproducing a bug, scraping a page), the agent uses{" "}
          {c("airgap_sandbox_run")} and uploads only the files the command needs.
        </p>
        <Prompt>use airgap and run the tests</Prompt>
        <Prompt>reproduce issue 88 in a sandbox</Prompt>
        <DocTable
          head={["Repo root contains", "Default image"]}
          rows={[
            [c("Cargo.toml"), c("rust:1-bookworm")],
            [c("package.json"), c("node:20-bookworm")],
            ["anything else", c("python:3.12-bookworm")],
          ]}
        />
        <p>
          Commands time out after 120 s by default and can run up to 1800 s. The first run for an image builds a template,
          so it is slower than the runs after it.
        </p>
      </>
    ),
  },
  {
    id: "issue-text",
    title: "Act on untrusted issue text",
    body: (
      <>
        <p>
          Issue bodies and comments can carry hidden instructions. Ask the agent to check the text first; it calls{" "}
          {c("airgap_scan_injection")} before it acts on anything in it.
        </p>
        <Prompt>fix the bug in issue 204, but scan it with airgap first</Prompt>
      </>
    ),
  },
  {
    id: "manifest",
    title: "Audit a dependency manifest",
    body: (
      <>
        <p>
          Before installing from a {c("package.json")}, {c("Cargo.toml")} or {c("requirements.txt")} you did not write,
          ask for a supply audit. It runs locally and needs no keys.
        </p>
        <Prompt>use airgap to audit this package.json before we install</Prompt>
        <p>
          By default {c("fail_on_supply_risk")} is true and {c("allow_install_scripts")} is false, so manifests with
          lifecycle hooks are refused unless you change those settings.
        </p>
      </>
    ),
  },
  {
    id: "scrub",
    title: "Scrub logs before sharing",
    body: (
      <>
        <p>
          Before pasting logs, diffs or environment dumps into an issue or chat, ask the agent to redact them.{" "}
          {c("airgap_redact_secrets")} runs locally and replaces API keys, bearer tokens and private key blocks with{" "}
          {c("[REDACTED]")}.
        </p>
        <Prompt>redact secrets from this CI log so I can post it</Prompt>
      </>
    ),
  },
  {
    id: "audit",
    title: "Run a security review",
    body: (
      <>
        <p>
          {c("airgap_audit_repo")} performs a defensive, static security review of a repository in six phases, using the
          microVM and GMI MaaS. It does not exploit anything.
        </p>
        <Prompt>use airgap to do a security audit of this repo</Prompt>
        <p>
          Read the{" "}
          <a href={LINKS.securityModel} target="_blank" rel="noopener noreferrer">
            security model
          </a>{" "}
          for what this boundary does and does not protect against.
        </p>
      </>
    ),
  },
];

export function GuidesPage() {
  return (
    <DocPage
      badge="Guides"
      title="Ask in plain words."
      lead="Short walkthroughs for the jobs airgap does most: verify a PR, test a patch, run tests, audit a manifest and scrub logs before you share them."
      sections={SECTIONS}
    />
  );
}
