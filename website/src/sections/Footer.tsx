import { ArrowRight, ArrowUpRight } from "lucide-react";
import { CLIENTS, FOOTER_COLUMNS, INSTALL, LINKS, VERSION, at } from "../content";
import { CopyButton, GithubIcon, Mark, Ticks } from "../lib";
import { GateStack } from "./GatesIso";
import { FluidGradientText } from "@/components/fluid-gradient-text";
import { scrollToTarget } from "../smooth";

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
          <a className="btn btn--accent" href={at(at("/docs"))}>
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

export function Footer() {
  const year = new Date().getFullYear();
  return (
    <footer className="site-footer">
      <div className="frame footer-frame">
        <div className="footer-top">
          <div className="footer-brand">
            <a className="brand" href={at(at("/#top"))} aria-label="airgap home">
              <Mark size={19} />
              <span className="brand-name">airgap</span>
            </a>
            <p>
              The microVM execution boundary for AI coding agents. Open source, written in Rust, dual-licensed MIT OR
              Apache-2.0.
            </p>
            <a className="btn btn--ghost btn--sm" href={LINKS.repo} target="_blank" rel="noopener noreferrer">
              <GithubIcon size={15} /> View on GitHub
            </a>
          </div>
          <nav className="footer-cols" aria-label="Footer">
            {FOOTER_COLUMNS.map((col) => (
              <div key={col.heading} className="footer-col">
                <p className="footer-heading">{col.heading}</p>
                <ul>
                  {col.links.map((l) => (
                    <li key={l.label}>
                      <a
                        href={l.href}
                        {...(l.external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
                      >
                        {l.label}
                        {l.external && <ArrowUpRight size={12} strokeWidth={1.8} />}
                      </a>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </nav>
        </div>
        <p className="footer-note">
          airgap is an independent project and is not an official GMI Cloud product. It uses{" "}
          <a href={LINKS.agentbox} target="_blank" rel="noopener noreferrer">
            GMI Cloud AgentBox v2
          </a>{" "}
          for sandboxes and GMI MaaS for inference. Sandboxes have outbound internet access; files you send leave your
          machine.
        </p>
        <div className="footer-bottom">
          <span>
            &copy; {year}{" "}
            <a href={LINKS.author} target="_blank" rel="noopener noreferrer">
              Dhruvil Mistry
            </a>
          </span>
          <span className="footer-status">
            <i aria-hidden="true" /> v{VERSION} &middot; 6 release targets
          </span>
          <a href="#top" onClick={(e) => { e.preventDefault(); scrollToTarget(0); }}>Back to top</a>
        </div>
        <div className="footer-word" aria-hidden="true">
          <FluidGradientText text="airgap" />
        </div>
      </div>
    </footer>
  );
}
