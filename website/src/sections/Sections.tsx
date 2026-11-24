import { useEffect, useState, type ReactNode } from "react";
import { Tabs } from "@base-ui/react/tabs";
import { Accordion } from "@base-ui/react/accordion";
import { CircleDot, Plus, ShieldCheck } from "lucide-react";
import { FAQ, INSTALL, ISOLATION_POINTS, SETUP_CLIENTS, type CodeLine, type SetupClient } from "../content";
import { Code, CopyButton, Reveal, Row, ScrollFloat, SectionHead, cx } from "../lib";
import { BoundaryScene } from "./Scene";
import { GatesIso } from "./GatesIso";

export function BoundarySection() {
  return (
    <Row index="01" id="boundary" labelledBy="boundary-title">
      <SectionHead
        id="boundary-title"
        badge="How it works"
        title="Every call crosses one boundary."
        muted="Watch what gets through."
        lead="Pick a scenario. The same pipeline runs each time; what changes is where it stops."
      />
      <Reveal>
        <ScrollFloat offset={16}>
          <BoundaryScene />
        </ScrollFloat>
      </Reveal>
    </Row>
  );
}

export function GatesSection() {
  return (
    <Row index="03" id="gates" labelledBy="gates-title">
      <SectionHead
        id="gates-title"
        badge="Security gates"
        title="Four gates. Each does one job."
        muted="Inspect every layer."
        lead="Every PR verification and sandbox run passes top to bottom through four controls. Click any layer in the stack to inspect it."
      />
      <Reveal>
        <GatesIso />
      </Reveal>
      <Reveal className="iso-points">
        {ISOLATION_POINTS.map((pt) => (
          <span key={pt.text} className={cx("iso-point", !pt.on && "iso-point--note")}>
            <CircleDot size={13} strokeWidth={2} /> {pt.text}
          </span>
        ))}
      </Reveal>
    </Row>
  );
}

const REPORT = [
  ["tool", "airgap_verify_pr"],
  ["target", "acme/api#142"],
  ["head", "a4f9c21"],
  ["injection", "clean"],
  ["supply", "no lifecycle hooks"],
  ["sandbox", "gmi.sandbox.medium \u00b7 2 vCPU \u00b7 4 GiB"],
  ["tests", "targeted, passed"],
  ["secrets", "scrubbed"],
] as const;

const JSON_LINES: CodeLine[] = [
  ["{"],
  ["  ", ["key", '"mcpServers"'], ": {"],
  ["    ", ["key", '"airgap"'], ": { ", ["key", '"command"'], ": ", ["str", '"airgap"'], ", ", ["key", '"args"'], ": [", ["str", '"--mcp"'], "] }"],
  ["  }"],
  ["}"],
];

function CmdLine({ cmd, prompt = "$", label }: { cmd: string; prompt?: string; label: string }) {
  return (
    <div className="cmdline">
      <span className="cmdline-prompt" aria-hidden="true">
        {prompt}
      </span>
      <code className="cmdline-code">{cmd}</code>
      <CopyButton value={cmd} label={label} />
    </div>
  );
}

function Step({ n, title, desc, aside, children }: { n: string; title: string; desc: ReactNode; aside?: ReactNode; children: ReactNode }) {
  return (
    <li className="step">
      <span className="step-n" aria-hidden="true">
        {n}
      </span>
      <div className="step-main">
        <div className="step-head">
          <h3 className="step-title">{title}</h3>
          {aside}
        </div>
        <p className="step-desc">{desc}</p>
        {children}
      </div>
    </li>
  );
}

function SetupSteps({ client }: { client: SetupClient }) {
  const [os, setOs] = useState<string>(INSTALL[0].value);
  const install = INSTALL.find((i) => i.value === os) ?? INSTALL[0];
  const register = client.flag ? `airgap setup ${client.flag} --yes` : null;
  const all = [install.command, register, "airgap doctor"].filter(Boolean).join("\n");

  return (
    <div className="setup-steps-wrap">
      <div className="setup-steps-head">
        <span>
          {client.name} &middot; 4 steps &middot; about a minute
        </span>
        <span className="setup-copyall">
          Copy all
          <CopyButton value={all} label={`Copy every ${client.name} setup command`} />
        </span>
      </div>
      <ol className="setup-steps">
        <Step
          n="01"
          title="Install the binary"
          desc="Prebuilt for Linux, macOS and Windows on x86_64 and ARM64. The installer checks checksums first."
          aside={
            <div className="os-seg" role="radiogroup" aria-label="Operating system">
              {INSTALL.map((i) => (
                <button
                  key={i.value}
                  type="button"
                  role="radio"
                  aria-checked={os === i.value}
                  className={cx("os-opt", os === i.value && "is-on")}
                  onClick={() => setOs(i.value)}
                >
                  {i.value === "sh" ? "macOS / Linux" : i.label}
                </button>
              ))}
            </div>
          }
        >
          <CmdLine cmd={install.command} prompt={install.prompt} label="Copy install command" />
        </Step>
        <Step
          n="02"
          title={`Register with ${client.name}`}
          desc={
            <>
              {client.json ? client.writes : <>Writes <code>{client.writes}</code>.</>}{" "}
              {client.json ? (
                <>
                  Add rules for your agent with <code>airgap setup --rules-only --yes</code>.
                </>
              ) : (
                "Does not call GMI or create compute."
              )}
            </>
          }
        >
          {register ? (
            <CmdLine cmd={register} label={`Copy ${client.name} setup command`} />
          ) : (
            <div className="cmdblock">
              <Code lines={JSON_LINES} />
              <span className="cmdblock-copy">
                <CopyButton value={client.json ?? ""} label="Copy MCP config" />
              </span>
            </div>
          )}
        </Step>
        <Step
          n="03"
          title="Check keys and connectivity"
          desc={
            <>
              Set <code>GMI_AGENTBOX_API_KEY</code> and <code>GMI_MAAS_API_KEY</code> first. Doctor is read-only and
              prints a fix for anything that fails.
            </>
          }
        >
          <CmdLine cmd="airgap doctor" label="Copy doctor command" />
        </Step>
        <Step n="04" title="Ask in plain words" desc="The routing rules pick the right airgap tool. You never have to name one.">
          <CmdLine cmd={client.prompt} prompt=">" label="Copy example prompt" />
        </Step>
      </ol>
    </div>
  );
}

export function SetupSection() {
  const [value, setValue] = useState(SETUP_CLIENTS[0].value);
  const [flash, setFlash] = useState(0);

  useEffect(() => {
    const pick = (e: Event) => {
      const tab = (e as CustomEvent<string>).detail;
      if (!SETUP_CLIENTS.some((s) => s.value === tab)) return;
      setValue(tab);
      setFlash((n) => n + 1);
    };
    window.addEventListener("airgap:client", pick);
    return () => window.removeEventListener("airgap:client", pick);
  }, []);

  return (
    <Row index="04" id="setup" labelledBy="setup-title" dark>
      <SectionHead
        id="setup-title"
        badge="Setup"
        title="One command per client."
        muted="Then ask in plain words."
        lead="Pick your agent. Every command has its own copy button, or copy them all at once."
      />
      <Reveal>
        <Tabs.Root
          key={flash ? `flash-${flash}` : "setup"}
          className={cx("setup", flash > 0 && "term--flash")}
          value={value}
          onValueChange={(v) => setValue(String(v))}
        >
          <Tabs.List className="setup-clients" aria-label="Choose your agent">
            {SETUP_CLIENTS.map((c) => (
              <Tabs.Tab key={c.value} value={c.value} className="setup-client">
                <img src={c.logo} alt="" width={16} height={16} className={cx(c.invert && "is-invert")} />
                {c.name}
              </Tabs.Tab>
            ))}
            <Tabs.Indicator className="setup-indicator" />
          </Tabs.List>
          <div className="setup-body">
            {SETUP_CLIENTS.map((c) => (
              <Tabs.Panel key={c.value} value={c.value} className="setup-panel">
                <SetupSteps client={c} />
              </Tabs.Panel>
            ))}
            <aside className="report setup-result" aria-label="Example result">
              <div className="report-head">
                <span>What comes back</span>
                <span className="report-badge">
                  <ShieldCheck size={13} strokeWidth={2} /> VERIFIED
                </span>
              </div>
              <dl className="report-dl">
                {REPORT.map(([k, v]) => (
                  <div key={k}>
                    <dt>{k}</dt>
                    <dd>{v}</dd>
                  </div>
                ))}
              </dl>
              <div className="setup-log">
                <p className="setup-log-head">Sandbox log, as your agent sees it</p>
                <pre>
                  <span className="dim">$ npm test</span>
                  {"\n"}
                  <span className="ok">✓</span> 38 passed, 0 failed
                  {"\n"}
                  <span className="dim">env</span> NPM_TOKEN=<span className="red">[REDACTED]</span>
                  {"\n"}
                  <span className="dim">vm </span> deleted after run
                </pre>
              </div>
              <p className="report-foot">Unsigned SHA-256 receipt over the inputs and output</p>
            </aside>
          </div>
        </Tabs.Root>
      </Reveal>
    </Row>
  );
}
export function FaqSection() {
  return (
    <Row index="05" id="faq" labelledBy="faq-title">
      <SectionHead id="faq-title" badge="FAQ" title="Straight answers." muted="Including the limits." />
      <Reveal>
        <Accordion.Root className="faq" defaultValue={[FAQ[0].q]}>
          {FAQ.map((f) => (
            <Accordion.Item key={f.q} value={f.q} className="faq-item">
              <Accordion.Header className="faq-h">
                <Accordion.Trigger className="faq-trigger">
                  {f.q}
                  <span className="faq-icon" aria-hidden="true">
                    <Plus size={16} strokeWidth={1.8} />
                  </span>
                </Accordion.Trigger>
              </Accordion.Header>
              <Accordion.Panel className="faq-panel" hiddenUntilFound>
                <p>{f.a}</p>
              </Accordion.Panel>
            </Accordion.Item>
          ))}
        </Accordion.Root>
      </Reveal>
    </Row>
  );
}
