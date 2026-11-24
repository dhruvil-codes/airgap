import { Mark } from "../lib";
import { GateStack } from "./GatesIso";

/** Static 1200x630 social card, rendered alone at /?og and captured as public/og.png. */
export function OgImage() {
  return (
    <div className="ogi" id="og-image">
      <div className="ogi-bg" aria-hidden="true" />
      <div className="ogi-copy">
        <span className="ogi-brand">
          <Mark size={34} />
          airgap
        </span>
        <h1 className="ogi-title">
          Let your agent
          <br />
          run anything.
        </h1>
        <p className="ogi-sub">Just not on your machine.</p>
        <div className="ogi-foot">
          <span className="ogi-cta">
            Get started at useairgap.dev <span aria-hidden="true">→</span>
          </span>
          <span className="ogi-meta">Open-source MCP server · MicroVM sandbox</span>
        </div>
      </div>
      <div className="ogi-art" aria-hidden="true">
        <GateStack active={2} idp="ogi" className="ogi-stack" />
      </div>
    </div>
  );
}
