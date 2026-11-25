import { ArrowUpRight } from "lucide-react";
import { FOOTER_COLUMNS, LINKS, VERSION, at } from "../content";
import { GithubIcon, Mark } from "../lib";
import { FluidGradientText } from "@/components/fluid-gradient-text";
import { scrollToTarget } from "../smooth";

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
