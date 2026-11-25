import { useEffect, useState } from "react";
import { m } from "motion/react";
import { EASE, cx } from "../lib";

/* ───────────────────────────────────────────────────────────────────────────
   iso-glow projection kernel (standard 30° isometric, pure vector)
   ─────────────────────────────────────────────────────────────────────────── */
const C = Math.cos(Math.PI / 6);
const S = Math.sin(Math.PI / 6);
type V3 = readonly [number, number, number];
type V2 = readonly [number, number];

const P = (x: number, y: number, z: number): V2 => [(x - y) * C, (x + y) * S - z];
const D = (x: number, y: number, z: number): V2 => [(x - y) * C, (x + y) * S - z];
const pt = (p: V2) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`;
const poly = (pts: V2[]) => pts.map(pt).join(" ");

const plane = (O: V3, U: V3, V: V3) => {
  const o = P(...O), u = D(...U), v = D(...V);
  return `matrix(${u[0].toFixed(4)} ${u[1].toFixed(4)} ${v[0].toFixed(4)} ${v[1].toFixed(4)} ${o[0].toFixed(2)} ${o[1].toFixed(2)})`;
};
const TOP = (x: number, y: number, z: number) => plane([x, y, z], [1, 0, 0], [0, 1, 0]);

/** Rounded isometric plate (continuous side wall — corners never notch) */
function roundedPlatePaths(x: number, y: number, z: number, dx: number, dy: number, dz: number, r: number, seg = 7) {
  const rr = Math.min(r, dx / 2 - 1, dy / 2 - 1);
  const arc = (cx: number, cy: number, a0: number, a1: number, z0: number) => {
    const out: V2[] = [];
    for (let i = 0; i <= seg; i++) {
      const a = a0 + ((a1 - a0) * i) / seg;
      out.push(P(cx + rr * Math.cos(a), cy + rr * Math.sin(a), z0));
    }
    return out;
  };
  const topRing = [
    ...arc(x + dx - rr, y + rr, -Math.PI / 2, 0, z + dz),
    ...arc(x + dx - rr, y + dy - rr, 0, Math.PI / 2, z + dz),
    ...arc(x + rr, y + dy - rr, Math.PI / 2, Math.PI, z + dz),
    ...arc(x + rr, y + rr, Math.PI, (3 * Math.PI) / 2, z + dz),
  ];
  // Visible front silhouette runs from rightmost tangent (angle ~ -pi/6) through bottom (+pi/2) to leftmost tangent (~ +7pi/6)
  const frontTop = [
    ...arc(x + dx - rr, y + rr, -Math.PI / 6, 0, z + dz),
    ...arc(x + dx - rr, y + dy - rr, 0, Math.PI / 2, z + dz),
    ...arc(x + rr, y + dy - rr, Math.PI / 2, Math.PI, z + dz),
    ...arc(x + rr, y + rr, Math.PI, (7 * Math.PI) / 6, z + dz),
  ];
  const frontBot = [
    ...arc(x + dx - rr, y + rr, -Math.PI / 6, 0, z),
    ...arc(x + dx - rr, y + dy - rr, 0, Math.PI / 2, z),
    ...arc(x + rr, y + dy - rr, Math.PI / 2, Math.PI, z),
    ...arc(x + rr, y + rr, Math.PI, (7 * Math.PI) / 6, z),
  ];
  const wall = [...frontTop, ...frontBot.slice().reverse()];
  const toD = (pts: V2[]) => `M ${pt(pts[0])} ${pts.slice(1).map((p) => `L ${pt(p)}`).join(" ")} Z`;

  // Hidden back-edge dashed wireframe (echoing Image 5's technical construction lines)
  const backTop = P(x, y, z + dz);
  const backBot = P(x, y, z);
  const backLeft = P(x, y + dy - rr, z);
  const backRight = P(x + dx - rr, y, z);

  return {
    topD: toD(topRing),
    wallD: toD(wall),
    backSpine: `M ${pt(backTop)} L ${pt(backBot)}`,
    backEdges: `M ${pt(backLeft)} L ${pt(backBot)} L ${pt(backRight)}`,
  };
}

/** Small sharp extruded isometric cube */
function miniCube(x: number, y: number, z: number, dx: number, dy: number, dz: number) {
  const top = poly([P(x, y, z + dz), P(x + dx, y, z + dz), P(x + dx, y + dy, z + dz), P(x, y + dy, z + dz)]);
  const left = poly([P(x, y + dy, z + dz), P(x + dx, y + dy, z + dz), P(x + dx, y + dy, z), P(x, y + dy, z)]);
  const right = poly([P(x + dx, y, z + dz), P(x + dx, y + dy, z + dz), P(x + dx, y + dy, z), P(x + dx, y, z)]);
  return { top, left, right };
}

type GateSpec = {
  id: string;
  num: string;
  name: string;
  stage: string;
  summary: string;
  facts: readonly [string, string, string];
  readout: {
    tool: string;
    check: string;
    verdict: string;
    detail: string;
  };
};

const GATES: GateSpec[] = [
  {
    id: "scan",
    num: "01",
    name: "Injection scan",
    stage: "Before compute",
    summary: "PR titles, issue bodies, comments and diffs are scanned before any microVM is created. Malicious prompts stop the run cold.",
    facts: ["Local heuristics + GMI MaaS", "Blocks before VM boot", "Zero compute spend on attacks"],
    readout: {
      tool: "airgap_scan_injection",
      check: "PR title, body & diff comments",
      verdict: "CLEAN — proceed to supply audit",
      detail: "Malicious instructions abort before sandbox creation",
    },
  },
  {
    id: "supply",
    num: "02",
    name: "Supply audit",
    stage: "Before install",
    summary: "Manifests are parsed for preinstall and postinstall hooks, git URLs and risky packages before npm, pip or cargo touches them.",
    facts: ["fail_on_supply_risk: true", "allow_install_scripts: false", "package.json · Cargo.toml · requirements.txt"],
    readout: {
      tool: "airgap_audit_supply",
      check: "lifecycle hooks & manifest pinning",
      verdict: "PASSED — no install hooks found",
      detail: "Refuses hostile scripts by default",
    },
  },
  {
    id: "vm",
    num: "03",
    name: "MicroVM isolation",
    stage: "During execution",
    summary: "Builds and tests run inside a fresh Linux microVM on GMI AgentBox v2, then the VM is deleted. Your host files stay local.",
    facts: ["gmi.sandbox.medium (2 vCPU / 4 GiB)", "120s default · up to 1800s", "Max 4 concurrent sandbox jobs"],
    readout: {
      tool: "airgap_sandbox_run",
      check: "fresh Linux VM · explicit upload only",
      verdict: "EXIT 0 — sandbox destroyed",
      detail: "Unsigned SHA-256 receipt attached",
    },
  },
  {
    id: "scrub",
    num: "04",
    name: "Secret scrub",
    stage: "Before return",
    summary: "Every line of stdout and stderr is scanned for tokens, API keys and private blocks, replaced with [REDACTED] before your agent reads it.",
    facts: ["Outputs [REDACTED]", "Covers stdout, stderr & diffs", "Safe to paste back to your agent"],
    readout: {
      tool: "airgap_redact_secrets",
      check: "API keys, bearer tokens, private keys",
      verdict: "SCRUBBED — 2 tokens replaced",
      detail: "Replaced inline with [REDACTED]",
    },
  },
];

// Plate geometry constants in world coordinates (top to bottom: z = 198, 134, 70, 6)
const W = 128;
const H = 128;
const THICK = 14;
const RAD = 14;
const Z_LEVELS = [198, 134, 70, 6] as const;

export function GateStack({
  active,
  onPick,
  idp = "gs",
  className = "iso-svg",
}: {
  active: number;
  onPick?: (i: number) => void;
  idp?: string;
  className?: string;
}) {
  const centerTop = P(W / 2, H / 2, Z_LEVELS[0] + THICK + 28);
  const centerBot = P(W / 2, H / 2, Z_LEVELS[3] - 10);
  return (
    <svg
      className={className}
      viewBox="-150 -275 300 415"
      role="img"
      aria-label={`Security gate ${GATES[active].num}: ${GATES[active].name}`}
    >
      <defs>
        {/* iso-glow filters: filterUnits="userSpaceOnUse" so straight lines never collapse */}
        <filter id={`${idp}-iso-glow`} filterUnits="userSpaceOnUse" x="-165" y="-290" width="330" height="445">
          <feGaussianBlur stdDeviation="4.5" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <filter id={`${idp}-iso-bloom`} filterUnits="userSpaceOnUse" x="-165" y="-290" width="330" height="445">
          <feGaussianBlur stdDeviation="14" />
        </filter>
        <linearGradient id={`${idp}-plate-top-dark`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#1b1e25" />
          <stop offset="100%" stopColor="#121419" />
        </linearGradient>
        <linearGradient id={`${idp}-plate-top-live`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#2a1912" />
          <stop offset="100%" stopColor="#181210" />
        </linearGradient>
        <linearGradient id={`${idp}-plate-wall-dark`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#0d0f13" />
          <stop offset="50%" stopColor="#15181f" />
          <stop offset="100%" stopColor="#0b0d11" />
        </linearGradient>
        <linearGradient id={`${idp}-plate-wall-live`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#26130b" />
          <stop offset="50%" stopColor="#461e0e" />
          <stop offset="100%" stopColor="#1e100a" />
        </linearGradient>
      </defs>

      {/* Subtle isometric ground shadow */}
      <path
        d={roundedPlatePaths(6, 6, -14, W - 12, H - 12, 0, RAD).topD}
        fill="rgba(0, 0, 0, 0.45)"
      />

      {/* Vertical signal conduit spine behind plates */}
      <line
        x1={centerTop[0]}
        y1={centerTop[1]}
        x2={centerBot[0]}
        y2={centerBot[1]}
        stroke="rgba(255,255,255,0.09)"
        strokeWidth="1"
        strokeDasharray="3 4"
      />

      {/* Travelling ember signal pulse along the vertical spine */}
      <line
        className="iso-spine-pulse"
        x1={centerTop[0]}
        y1={centerTop[1]}
        x2={centerBot[0]}
        y2={centerBot[1]}
        stroke="#ff5714"
        strokeWidth="2"
        strokeLinecap="round"
        filter={`url(#${idp}-iso-glow)`}
      />

      {/* Render plates bottom -> top (painter's algorithm: index 3, 2, 1, 0) */}
      {[3, 2, 1, 0].map((idx) => {
        const z = Z_LEVELS[idx];
        const isLive = idx === active;
        const plate = roundedPlatePaths(0, 0, z, W, H, THICK, RAD);
        const g = GATES[idx];
        const labelPt = P(W + 10, H / 2, z + THICK / 2);

        return (
          <m.g
            key={g.id}
            className={cx("iso-plate", isLive && "is-live")}
            animate={{ y: isLive ? -7 : 0 }}
            transition={{ duration: 0.42, ease: EASE }}
            onClick={onPick ? () => onPick(idx) : undefined}
              onMouseEnter={onPick ? () => onPick(idx) : undefined}
              style={onPick ? { cursor: "pointer" } : undefined}
          >
            {/* Ember bloom halo ONLY when live */}
            {isLive && (
              <path
                d={plate.topD}
                fill="rgba(255, 87, 20, 0.28)"
                filter={`url(#${idp}-iso-bloom)`}
              />
            )}

            {/* Continuous rounded side wall */}
            <path
              d={plate.wallD}
              fill={isLive ? `url(#${idp}-plate-wall-live)` : `url(#${idp}-plate-wall-dark)`}
              stroke={isLive ? "rgba(255, 87, 20, 0.65)" : "rgba(255, 255, 255, 0.14)"}
              strokeWidth={isLive ? 1.1 : 0.85}
            />

            {/* Dashed technical hidden back edges (like Image 5) */}
            <path
              d={plate.backEdges}
              fill="none"
              stroke={isLive ? "rgba(255, 87, 20, 0.25)" : "rgba(255, 255, 255, 0.06)"}
              strokeWidth="0.75"
              strokeDasharray="2.5 3"
            />

            {/* Top deck */}
            <path
              d={plate.topD}
              fill={isLive ? `url(#${idp}-plate-top-live)` : `url(#${idp}-plate-top-dark)`}
              stroke={isLive ? "#ff5714" : "rgba(255, 255, 255, 0.22)"}
              strokeWidth={isLive ? 1.35 : 0.9}
              filter={isLive ? `url(#${idp}-iso-glow)` : undefined}
            />

            {/* ── Layer-specific extruded details on top face ── */}

            {/* Layer 0 (Top): 4 floating input sub-blocks (echoing Image 5's 4-cube top!) */}
            {idx === 0 && (
              <g>
                {[
                  [20, 20],
                  [72, 20],
                  [20, 72],
                  [72, 72],
                ].map(([bx, by], bi) => {
                  const c = miniCube(bx, by, z + THICK + 1, 36, 36, 12);
                  const hot = isLive && bi === 1;
                  return (
                    <g key={bi}>
                      <polygon
                        points={c.left}
                        fill={hot ? "#c7400d" : isLive ? "#321b12" : "#151820"}
                        stroke={isLive ? "rgba(255,87,20,0.6)" : "rgba(255,255,255,0.18)"}
                        strokeWidth="0.8"
                      />
                      <polygon
                        points={c.right}
                        fill={hot ? "#9c3108" : isLive ? "#26140d" : "#101218"}
                        stroke={isLive ? "rgba(255,87,20,0.6)" : "rgba(255,255,255,0.18)"}
                        strokeWidth="0.8"
                      />
                      <polygon
                        points={c.top}
                        fill={hot ? "#ff5714" : isLive ? "#452113" : "#1e222b"}
                        stroke={isLive ? "#ff824d" : "rgba(255,255,255,0.28)"}
                        strokeWidth="0.9"
                        filter={hot ? `url(#${idp}-iso-glow)` : undefined}
                      />
                    </g>
                  );
                })}
              </g>
            )}

            {/* Layer 1: Supply audit manifest tracks on top plane */}
            {idx === 1 && (
              <g transform={TOP(22, 22, z + THICK + 0.5)}>
                <rect
                  x="0"
                  y="0"
                  width="84"
                  height="84"
                  rx="8"
                  fill={isLive ? "rgba(255,87,20,0.08)" : "rgba(255,255,255,0.02)"}
                  stroke={isLive ? "rgba(255,87,20,0.5)" : "rgba(255,255,255,0.14)"}
                  strokeWidth="1"
                />
                {[18, 36, 54, 68].map((ry, ri) => (
                  <line
                    key={ry}
                    x1="14"
                    y1={ry}
                    x2={ri === 1 ? 66 : ri === 3 ? 48 : 70}
                    y2={ry}
                    stroke={isLive && ri === 1 ? "#ff5714" : isLive ? "rgba(255,165,120,0.55)" : "rgba(255,255,255,0.22)"}
                    strokeWidth={ri === 1 ? 4 : 2.5}
                    strokeLinecap="round"
                    filter={isLive && ri === 1 ? `url(#${idp}-iso-glow)` : undefined}
                  />
                ))}
              </g>
            )}

            {/* Layer 2: MicroVM core cube inside recessed bay */}
            {idx === 2 && (() => {
              const core = miniCube(42, 42, z + THICK + 1, 44, 44, 16);
              return (
                <g>
                  <g transform={TOP(24, 24, z + THICK + 0.4)}>
                    <rect
                      x="0"
                      y="0"
                      width="80"
                      height="80"
                      rx="8"
                      fill="none"
                      stroke={isLive ? "rgba(255,87,20,0.45)" : "rgba(255,255,255,0.12)"}
                      strokeDasharray="4 4"
                      strokeWidth="1"
                    />
                  </g>
                  <polygon
                    points={core.left}
                    fill={isLive ? "#b83a0b" : "#181c24"}
                    stroke={isLive ? "#ff7a42" : "rgba(255,255,255,0.22)"}
                    strokeWidth="0.9"
                  />
                  <polygon
                    points={core.right}
                    fill={isLive ? "#8c2a07" : "#12151c"}
                    stroke={isLive ? "#ff7a42" : "rgba(255,255,255,0.22)"}
                    strokeWidth="0.9"
                  />
                  <polygon
                    points={core.top}
                    fill={isLive ? "#ff5714" : "#222732"}
                    stroke={isLive ? "#ffb08c" : "rgba(255,255,255,0.3)"}
                    strokeWidth="1"
                    filter={isLive ? `url(#${idp}-iso-glow)` : undefined}
                  />
                </g>
              );
            })()}

            {/* Layer 3 (Bottom): Secret scrub verified checkmark plate (echoing Image 5's bottom checkmark!) */}
            {idx === 3 && (
              <g transform={TOP(26, 26, z + THICK + 0.6)}>
                <circle
                  cx="38"
                  cy="38"
                  r="28"
                  fill={isLive ? "rgba(255,87,20,0.12)" : "rgba(255,255,255,0.03)"}
                  stroke={isLive ? "#ff5714" : "rgba(255,255,255,0.2)"}
                  strokeWidth={isLive ? 2 : 1.2}
                  filter={isLive ? `url(#${idp}-iso-glow)` : undefined}
                />
                <path
                  d="M 24 39 L 34 49 L 54 27"
                  fill="none"
                  stroke={isLive ? "#ff824d" : "rgba(255,255,255,0.42)"}
                  strokeWidth="4.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  filter={isLive ? `url(#${idp}-iso-glow)` : undefined}
                />
              </g>
            )}

            {/* Right-edge callout index tag */}
            <g transform={`translate(${labelPt[0].toFixed(1)}, ${labelPt[1].toFixed(1)})`}>
              <line
                x1="-4"
                y1="0"
                x2="14"
                y2="0"
                stroke={isLive ? "#ff5714" : "rgba(255,255,255,0.16)"}
                strokeWidth="1"
              />
              <text
                x="19"
                y="3.5"
                className={cx("iso-tag", isLive && "is-live")}
              >
                {g.num}
              </text>
            </g>
          </m.g>
        );
      })}
    </svg>
  );
}

export function GatesIso() {
  const [active, setActive] = useState(0);
  const [auto, setAuto] = useState(true);

  useEffect(() => {
    if (!auto) return;
    const id = window.setInterval(() => {
      setActive((i) => (i + 1) % GATES.length);
    }, 3600);
    return () => window.clearInterval(id);
  }, [auto]);

  const select = (i: number) => {
    setAuto(false);
    setActive(i);
  };

  const gate = GATES[active];
  return (
    <div
      className="iso-gates"
      tabIndex={0}
      aria-label="Interactive security gates stack"
      onKeyDown={(e) => {
        if (e.key === "ArrowDown" || e.key === "ArrowRight") {
          e.preventDefault();
          select((active + 1) % GATES.length);
        } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
          e.preventDefault();
          select((active + GATES.length - 1) % GATES.length);
        } else if (e.key >= "1" && e.key <= "4") {
          select(Number(e.key) - 1);
        }
      }}
    >
      {/* ── Left: iso-glow interactive 4-plate isometric stack ── */}
      <div className="iso-stage">
        <div className="iso-stage-bar">
          <span className="iso-stage-dot" />
          <span>EXECUTION PIPELINE</span>
          <span className="iso-stage-hint">Click a layer or press 1–4</span>
        </div>

        <GateStack
          active={active}
          onPick={(i) => {
            setAuto(false);
            setActive(i);
          }}
        />
      </div>

      {/* ── Right: Interactive gate selector + live readout ── */}
      <div className="iso-panel">
        <div className="iso-list" role="tablist" aria-label="Security gates">
          {GATES.map((g, i) => {
            const isLive = i === active;
            return (
              <button
                key={g.id}
                type="button"
                role="tab"
                aria-selected={isLive}
                className={cx("iso-row", isLive && "is-live")}
                onClick={() => select(i)}
              >
                <span className="iso-row-num">{g.num}</span>
                <span className="iso-row-main">
                  <span className="iso-row-title">{g.name}</span>
                  <span className="iso-row-stage">{g.stage}</span>
                </span>
                <span className="iso-row-dot" aria-hidden="true" />
              </button>
            );
          })}
        </div>

        <m.div
          key={gate.id}
          className="iso-detail"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.24, ease: EASE }}
        >
          <p className="iso-summary">{gate.summary}</p>
          <ul className="iso-facts">
            {gate.facts.map((f) => (
              <li key={f}>{f}</li>
            ))}
          </ul>

          <div className="iso-readout">
            <div className="iso-readout-row">
              <span className="iso-rk">tool</span>
              <span className="iso-rv iso-rv--accent">{gate.readout.tool}</span>
            </div>
            <div className="iso-readout-row">
              <span className="iso-rk">inspects</span>
              <span className="iso-rv">{gate.readout.check}</span>
            </div>
            <div className="iso-readout-row">
              <span className="iso-rk">verdict</span>
              <span className="iso-rv iso-rv--ok">{gate.readout.verdict}</span>
            </div>
          </div>
        </m.div>
      </div>
    </div>
  );
}
