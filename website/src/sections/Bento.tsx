import { useEffect, useState } from "react";
import { m, useReducedMotion } from "motion/react";
import { Cloud, FileCode2, KeyRound, Lock, PackageSearch, ScanText, Search, Terminal } from "lucide-react";
import { NumberRoll, Reveal, Row, SectionHead, cx } from "../lib";
import { at } from "../content";

function useCycle(length: number, ms: number) {
  const reduce = useReducedMotion();
  const [i, setI] = useState(0);
  useEffect(() => {
    if (reduce) return;
    const id = window.setInterval(() => setI((n) => (n + 1) % length), ms);
    return () => window.clearInterval(id);
  }, [length, ms, reduce]);
  return i;
}

/* ── Card 1: hub routes to every client, cursor hops between them ── */
const CLIENT_TILES = [
  { x: 42, y: 112, logo: at("/logos/claude-code.svg"), name: "Claude Code" },
  { x: 108, y: 134, logo: at("/logos/cursor.svg"), name: "Cursor" },
  { x: 192, y: 134, logo: at("/logos/vscode.svg"), name: "VS Code" },
  { x: 258, y: 112, logo: at("/logos/factory.svg"), name: "Factory Droid" },
];

function HubMark({ cx: x, cy: y, size }: { cx: number; cy: number; size: number }) {
  const k = size / 64;
  return (
    <g transform={`translate(${x - size / 2} ${y - size / 2}) scale(${k})`} fill="#fff">
      <path d="M35.4 10L52.6 20Q56 22 52.6 24L35.4 34Q32 36 28.6 34L11.4 24Q8 22 11.4 20L28.6 10Q32 8 35.4 10Z" />
      <path d="M11.4 32L28.6 42Q32 44 35.4 42L52.6 32Q56 30 56 33.8V38.8Q56 42 52.6 44L35.4 54Q32 56 28.6 54L11.4 44Q8 42 8 38.8V33.8Q8 30 11.4 32Z" opacity={0.78} />
    </g>
  );
}

function ClientsVisual() {
  const active = useCycle(CLIENT_TILES.length, 1700);
  const t = CLIENT_TILES[active];
  return (
    <svg className="bv-svg" viewBox="0 0 300 176" aria-hidden="true">
      <defs>
        <linearGradient id="bv-hub" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ff7a45" />
          <stop offset="1" stopColor="#ff5714" />
        </linearGradient>
        <filter id="bv-shadow" x="-40%" y="-40%" width="180%" height="180%">
          <feDropShadow dx="0" dy="6" stdDeviation="6" floodColor="#111110" floodOpacity="0.08" />
        </filter>
        <filter id="bv-glow" x="-60%" y="-60%" width="220%" height="220%">
          <feDropShadow dx="0" dy="8" stdDeviation="8" floodColor="#ff5714" floodOpacity="0.35" />
        </filter>
      </defs>
      <path className="bv-arc" d="M6 170 Q150 30 294 170" />
      {CLIENT_TILES.map((c, i) => {
        const d = `M150 68 C150 96, ${c.x} 82, ${c.x} ${c.y - 20}`;
        return (
          <g key={c.name}>
            <path className="bv-track" d={d} />
            <path className={cx("bv-flow", i === active && "is-on")} d={d} pathLength={100} />
          </g>
        );
      })}
      <rect className="bv-ping" x="124" y="16" width="52" height="52" rx="15" />
      <rect x="124" y="16" width="52" height="52" rx="15" fill="url(#bv-hub)" filter="url(#bv-glow)" />
      <HubMark cx={150} cy={42} size={38} />
      {CLIENT_TILES.map((c, i) => (
        <g key={c.name} className={cx("bv-tile", i === active && "is-on")} transform={`translate(${c.x - 20} ${c.y - 20})`}>
          <rect width="40" height="40" rx="11" filter="url(#bv-shadow)" />
          <image href={c.logo} x="10" y="10" width="20" height="20" />
        </g>
      ))}
      <m.g
        initial={false}
        animate={{ x: t.x + 6, y: t.y + 4 }}
        transition={{ type: "spring", stiffness: 140, damping: 18 }}
      >
        <path
          d="M0 0 L0 15 L4 11.4 L7 18 L9.6 16.9 L6.7 10.4 L11.8 10.4 Z"
          fill="#111110"
          stroke="#fff"
          strokeWidth="1.2"
          strokeLinejoin="round"
        />
      </m.g>
    </svg>
  );
}

/* ── Card 3: host secrets on an arc, only the file spoke flows to the VM ── */
const ARC_TILES = [
  { a: 160, Icon: KeyRound, send: false },
  { a: 115, Icon: FileCode2, send: true },
  { a: 65, Icon: Terminal, send: false },
  { a: 20, Icon: Cloud, send: false },
].map((t) => ({
  ...t,
  x: 150 + 112 * Math.cos((t.a * Math.PI) / 180),
  y: 156 - 112 * Math.sin((t.a * Math.PI) / 180),
}));

function ArcVisual() {
  const reduce = useReducedMotion();
  const arc = "M38 156 A112 112 0 0 1 262 156";
  return (
    <svg className="bv-svg" viewBox="0 0 300 176" aria-hidden="true">
      <path className="bv-arc bv-arc--solid" d={arc} />
      <path className="bv-arc" d="M80 156 A70 70 0 0 1 220 156" />
      <path className="bv-arc-flow" d={arc} pathLength={100} />
      {ARC_TILES.map((t) => {
        const d = `M150 134 L${t.x.toFixed(1)} ${(t.y + 18).toFixed(1)}`;
        return (
          <g key={t.a}>
            <path className="bv-track" d={d} />
            {t.send && <path className="bv-flow bv-flow--in is-on" d={`M${t.x.toFixed(1)} ${(t.y + 18).toFixed(1)} L150 134`} pathLength={100} />}
          </g>
        );
      })}
      {!reduce && (
        <circle r="3" fill="#ff5714">
          <animateMotion dur="5s" repeatCount="indefinite" path={arc} />
        </circle>
      )}
      {ARC_TILES.map(({ a, x, y, Icon, send }, i) => (
        <g key={a} className={cx("bv-soft", send && "is-send")} style={{ animationDelay: `${i * 0.6}s` }} transform={`translate(${(x - 18).toFixed(1)} ${(y - 18).toFixed(1)})`}>
          <rect width="36" height="36" rx="10" />
          <Icon x={9} y={9} width={18} height={18} strokeWidth={1.7} />
        </g>
      ))}
      <rect className="bv-ping" x="126" y="112" width="48" height="48" rx="14" />
      <rect x="126" y="112" width="48" height="48" rx="14" fill="url(#bv-hub2)" filter="url(#bv-glow2)" />
      <defs>
        <linearGradient id="bv-hub2" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ff7a45" />
          <stop offset="1" stopColor="#ff5714" />
        </linearGradient>
        <filter id="bv-glow2" x="-60%" y="-60%" width="220%" height="220%">
          <feDropShadow dx="0" dy="8" stdDeviation="8" floodColor="#ff5714" floodOpacity="0.35" />
        </filter>
      </defs>
      <Lock x={139} y={125} width={22} height={22} color="#fff" strokeWidth={1.8} />
    </svg>
  );
}

/* ── Compact card glyphs ── */
function OsGlyph() {
  return (
    <svg className="bv-glyph" viewBox="0 0 88 32" aria-hidden="true">
      {[6, 16, 26].map((y, i) => (
        <g key={y}>
          <path className="bv-track" d={`M10 ${y} C34 ${y}, 40 16, 62 16`} />
          <path className="bv-flow is-on" style={{ animationDelay: `${i * 0.45}s` }} d={`M10 ${y} C34 ${y}, 40 16, 62 16`} pathLength={100} />
          <circle cx="7" cy={y} r="3" className="bv-node" />
        </g>
      ))}
      <rect x="62" y="6" width="20" height="20" rx="6" fill="#111110" />
      <path d="M67 13 l3 3 l-3 3 M72 19 h4" stroke="#fff" strokeWidth="1.4" fill="none" strokeLinecap="round" />
    </svg>
  );
}

function ImageGlyph() {
  return (
    <div className="bv-langs" aria-hidden="true">
      {["node", "python", "rust"].map((l, i) => (
        <span key={l} style={{ animationDelay: `${i * 1.2}s` }}>
          {l}
        </span>
      ))}
    </div>
  );
}

/* ── Row 2 data ── */
const TOOL_ROWS = [
  ["airgap_verify_pr", "airgap_sandbox_run", "airgap_audit_repo", "airgap_configure"],
  ["airgap_scan_injection", "airgap_verify_patch", "airgap_redact_secrets", "airgap_audit_supply"],
  ["airgap_audit_supply", "airgap_configure", "airgap_verify_pr", "airgap_scan_injection"],
];

const HOST_ROWS = [
  ["Injection scan", "Before VM"],
  ["Supply audit", "Before install"],
  ["Secret scrub", "Before return"],
];

const POLICY_ROWS = [
  { tool: "airgap_verify_pr", Icon: ScanText, gate: "Injection", policy: "BLOCK_MALICIOUS" },
  { tool: "airgap_sandbox_run", Icon: PackageSearch, gate: "Supply", policy: "NO_INSTALL_HOOKS" },
  { tool: "airgap_redact_secrets", Icon: KeyRound, gate: "Scrub", policy: "[REDACTED]" },
];

function PolicyVisual() {
  const active = useCycle(POLICY_ROWS.length, 1900);
  return (
    <div className="bp" aria-hidden="true">
      <div className="bp-frame">
        <div className="bp-search">
          <Search size={14} strokeWidth={1.8} />
          <span className="bp-typing">fail_on_supply_risk</span>
          <i className="bp-caret" />
        </div>
        <div className="bp-head">
          <span className="bp-box" />
          <span>Tool</span>
          <span>Gate</span>
          <span>Policy</span>
        </div>
      </div>
      <div className="bp-rows">
        {POLICY_ROWS.map((r, i) => (
          <div key={r.tool} className={cx("bp-row", i === active && "is-on")} style={{ ["--step" as string]: i }}>
            <span className={cx("bp-box", i <= active && "is-checked")}>
              <svg viewBox="0 0 12 12">
                <path d="M2.5 6.2 L5 8.6 L9.6 3.6" />
              </svg>
            </span>
            <span className="bp-tool">{r.tool}</span>
            <span className="bp-gate">
              <r.Icon size={14} strokeWidth={1.8} />
              {r.gate}
            </span>
            <span className="bp-policy">{r.policy}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function Copy({ title, body }: { title: string; body: string }) {
  return (
    <div className="bc-copy">
      <h3>{title}</h3>
      <p>{body}</p>
    </div>
  );
}

export function BentoSection() {
  return (
    <Row index="02" id="problem" labelledBy="problem-title">
      <SectionHead
        id="problem-title"
        badge="The problem & the fix"
        title="Your agent runs untrusted code."
        muted="Keep it off your machine."
        lead="Every install, build and test runs inside a disposable Linux microVM on GMI Cloud, away from your SSH keys and cloud tokens."
      />

      <div className="bento">
        <div className="bento-row bento-row--3">
          <Reveal className="bc bc--visual">
            <div className="bc-vis">
              <ClientsVisual />
            </div>
            <Copy
              title="Works with the agent you use"
              body="One command registers airgap with Claude Code, Cursor, VS Code or Factory Droid. Any stdio MCP client works too."
            />
          </Reveal>

          <div className="bento-col">
            <Reveal className="bc bc--compact" delay={0.05}>
              <OsGlyph />
              <Copy title="Same Linux from any OS" body="Windows, macOS or Linux host. Every run gets the same fresh Linux microVM." />
            </Reveal>
            <Reveal className="bc bc--compact" delay={0.1}>
              <ImageGlyph />
              <Copy title="Node, Python or Rust" body="airgap picks the image from your files. Templates cache after the first run." />
            </Reveal>
          </div>

          <Reveal className="bc bc--visual" delay={0.14}>
            <div className="bc-vis">
              <ArcVisual />
            </div>
            <Copy
              title="Your secrets stay home"
              body="SSH keys and cloud tokens stay on your machine. Only the files you pass in a call cross the boundary."
            />
          </Reveal>
        </div>

        <div className="bento-row bento-row--2">
          <div className="bento-col">
            <Reveal className="bc bc--tools" delay={0.04}>
              <div className="bt">
                <div className="bt-rows" aria-hidden="true">
                  {TOOL_ROWS.map((row, i) => (
                    <div key={i} className="bt-track" style={{ animationDuration: `${26 + i * 6}s`, animationDirection: i % 2 ? "reverse" : "normal" }}>
                      {[...row, ...row, ...row, ...row].map((t, j) => (
                        <span key={j}>{t}</span>
                      ))}
                    </div>
                  ))}
                </div>
                <div className="bt-stat">
                  <strong>
                    <NumberRoll value="8" />
                  </strong>
                  <span>MCP tools</span>
                </div>
              </div>
              <Copy
                title="One binary, every job"
                body="Verify PRs and patches, run commands, scan text, audit manifests and scrub secrets from a single MCP server."
              />
            </Reveal>

            <Reveal className="bc bc--host" delay={0.1}>
              <div className="bh">
                <div className="bt-stat bt-stat--static">
                  <strong>0</strong>
                  <span>Host runs</span>
                </div>
                <div className="bh-table" aria-hidden="true">
                  {HOST_ROWS.map(([k, v], i) => (
                    <div key={k} className="bh-row">
                      <span>{k}</span>
                      <span className="bh-val">
                        {v}
                        <i className="bh-check" style={{ animationDelay: `${i * 0.5}s` }}>
                          <svg viewBox="0 0 12 12">
                            <path d="M2.5 6.2 L5 8.6 L9.6 3.6" />
                          </svg>
                        </i>
                      </span>
                    </div>
                  ))}
                </div>
              </div>
              <Copy
                title="Nothing runs on your host"
                body="Installs, builds and tests run in the VM. If a gate fails, the run stops before any compute starts."
              />
            </Reveal>
          </div>

          <Reveal className="bc bc--policy" delay={0.16}>
            <PolicyVisual />
            <Copy
              title="Safe defaults on every call"
              body="fail_on_supply_risk is on and install scripts are off by default. Every run returns scrubbed logs and an unsigned SHA-256 receipt."
            />
          </Reveal>
        </div>
      </div>
    </Row>
  );
}
