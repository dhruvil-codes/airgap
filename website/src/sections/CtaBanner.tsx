import { ArrowRight } from "lucide-react";
import { CLIENTS, INSTALL, LINKS, VERSION, at } from "../content";
import { CopyButton, GithubIcon, Mark, Ticks } from "../lib";
import { GateStack } from "./GatesIso";

export function OgCard() {
  return (
    <div className="og" id="og-card">
      <Ticks />
      <div className="og-grid" aria-hidden="true" />
      <div className="og-copy">
        <span className="og-brand">
          <Mark size={22} />
          airgap
          <span className="og-ver">v{VERSION}</span>
        </span>
        <h2 id="cta-title" className="og-title">
          Let your agent run anything.
          <br />
          <span>Just not on your machine.</span>
        </h2>
        <p className="og-sub">
          Open-source MCP server that runs installs, builds and tests in disposable Linux microVMs. Injection scan,
          supply audit and secret scrub on every call.
        </p>
        <div className="og-cmd">
          <span aria-hidden="true">$</span>
          <code>{INSTALL[0].command}</code>
          <CopyButton value={INSTALL[0].command} label="Copy install command" />
        </div>
        <div className="og-actions">
          <a className="btn btn--accent" href={at("/docs")}>
            Get started <ArrowRight size={15} strokeWidth={1.8} />
          </a>
          <a className="btn btn--on-dark" href={LINKS.repo} target="_blank" rel="noopener noreferrer">
            <GithubIcon size={15} /> dhruvil-codes/airgap
          </a>
        </div>
        <ul className="og-clients" aria-label="Works with">
          {CLIENTS.map((c) => (
            <li key={c.name}>
              <img src={c.logo} alt="" width={14} height={14} />
              {c.name === "Any stdio MCP client" ? "Any MCP client" : c.name}
            </li>
          ))}
        </ul>
      </div>
      <div className="og-art" aria-hidden="true">
        <GateStack active={1} idp="og" className="og-stack" />
      </div>
      <span className="og-foot" aria-hidden="true">
        MIT OR Apache-2.0 &nbsp;&middot;&nbsp; 8 MCP tools &nbsp;&middot;&nbsp; GMI AgentBox v2
      </span>
    </div>
  );
}

export function CtaBanner() {
  return (
    <section className="row cta-row" aria-labelledby="cta-title">
      <div className="frame cta-frame">
        <OgCard />
      </div>
    </section>
  );
}
