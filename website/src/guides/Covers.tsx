import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { GitPullRequest, Globe, KeyRound, PackageSearch, ScanText, Send, ShieldCheck } from "lucide-react";

/* Covers share the bento visual language (bv-* classes in site.css): white line tiles, a soft orange tile for
   the active step, the gradient hub with the airgap mark, faint tracks with an orange flow. 320x168 canvas. */

// Geist Mono advances 0.6em; .gd-art text adds 0.02em letter-spacing. Used to size pills to their label.
const MONO_ADVANCE = 0.62;

function Defs({ p }: { p: string }) {
  return (
    <defs>
      <linearGradient id={`${p}-hub`} x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stopColor="#ff7a45" />
        <stop offset="1" stopColor="#ff5714" />
      </linearGradient>
      <filter id={`${p}-glow`} x="-60%" y="-60%" width="220%" height="220%">
        <feDropShadow dx="0" dy="8" stdDeviation="8" floodColor="#ff5714" floodOpacity="0.32" />
      </filter>
      <filter id={`${p}-shadow`} x="-40%" y="-40%" width="180%" height="180%">
        <feDropShadow dx="0" dy="6" stdDeviation="6" floodColor="#111110" floodOpacity="0.08" />
      </filter>
    </defs>
  );
}

function Hub({ p, x, y, s = 48 }: { p: string; x: number; y: number; s?: number }) {
  const r = s * 0.29;
  const m = s * 0.73;
  return (
    <g>
      <rect className="bv-ping" x={x - s / 2} y={y - s / 2} width={s} height={s} rx={r} />
      <rect x={x - s / 2} y={y - s / 2} width={s} height={s} rx={r} fill={`url(#${p}-hub)`} filter={`url(#${p}-glow)`} />
      <g transform={`translate(${x - m / 2} ${y - m / 2}) scale(${m / 64})`} fill="#fff">
        <path d="M35.4 10L52.6 20Q56 22 52.6 24L35.4 34Q32 36 28.6 34L11.4 24Q8 22 11.4 20L28.6 10Q32 8 35.4 10Z" />
        <path
          d="M11.4 32L28.6 42Q32 44 35.4 42L52.6 32Q56 30 56 33.8V38.8Q56 42 52.6 44L35.4 54Q32 56 28.6 54L11.4 44Q8 42 8 38.8V33.8Q8 30 11.4 32Z"
          opacity={0.78}
        />
      </g>
    </g>
  );
}

function Tile({ p, x, y, Icon, soft, s = 36 }: { p: string; x: number; y: number; Icon: LucideIcon; soft?: boolean; s?: number }) {
  const i = s * 0.5;
  return (
    <g className={soft ? "bv-soft" : "bv-tile"} transform={`translate(${x - s / 2} ${y - s / 2})`}>
      <rect width={s} height={s} rx={s * 0.28} filter={soft ? undefined : `url(#${p}-shadow)`} />
      <Icon x={(s - i) / 2} y={(s - i) / 2} width={i} height={i} strokeWidth={1.7} />
    </g>
  );
}

type PillTone = "plain" | "ok" | "bad" | "accent" | "ink" | "hot";

function pillWidth(label: string, size: number, dot: boolean) {
  return label.length * size * MONO_ADVANCE + (dot ? 26 : 16);
}

function Pill({
  x,
  y,
  label,
  tone = "plain",
  dot = false,
  size = 10,
  anchor = "middle",
}: {
  x: number;
  y: number;
  label: string;
  tone?: PillTone;
  dot?: boolean;
  size?: number;
  anchor?: "start" | "middle";
}) {
  const w = pillWidth(label, size, dot);
  const h = size * 2;
  const x0 = anchor === "middle" ? x - w / 2 : x;
  return (
    <g className={`gd-pill gd-pill--${tone}`}>
      <rect x={x0} y={y - h / 2} width={w} height={h} rx={h / 2} />
      {dot && <circle cx={x0 + 11} cy={y} r={2.8} />}
      <text x={x0 + (dot ? 19 : 8)} y={y + size * 0.36} fontSize={size}>
        {label}
      </text>
    </g>
  );
}

function Flow({ d, on = true }: { d: string; on?: boolean }) {
  return (
    <>
      <path className="bv-track" d={d} />
      {on && <path className="bv-flow is-on" d={d} pathLength={100} />}
    </>
  );
}

function T({ x, y, children, size = 8.5, tone, anchor }: { x: number; y: number; children: ReactNode; size?: number; tone?: string; anchor?: "start" | "middle" | "end" }) {
  return (
    <text className={tone ? `gd-t gd-t--${tone}` : "gd-t"} x={x} y={y} fontSize={size} textAnchor={anchor}>
      {children}
    </text>
  );
}

function Dots({ x, y }: { x: number; y: number }) {
  return (
    <g className="gd-term-dots">
      {[0, 1, 2].map((i) => (
        <circle key={i} cx={x + i * 9} cy={y} r={2.4} />
      ))}
    </g>
  );
}

/* ── one composition per guide ── */

function VerifyPr({ p }: { p: string }) {
  return (
    <>
      <Flow d="M64 84 L136 84" />
      <Flow d="M184 84 L226 84" />
      <circle className="bv-node" cx="90" cy="84" r="3.2" />
      <circle className="bv-node" cx="112" cy="84" r="3.2" />
      <T x={90} y={72} anchor="middle">scan</T>
      <T x={112} y={102} anchor="middle">audit</T>
      <Pill x={46} y={42} label="acme/api#142" />
      <Tile p={p} x={46} y={84} Icon={GitPullRequest} />
      <Hub p={p} x={160} y={84} />
      <Pill x={160} y={134} label="sha a1f3c9e" />
      <Pill x={226} y={84} label="VERIFIED" tone="ok" dot anchor="start" />
    </>
  );
}

function VerifyPatch({ p }: { p: string }) {
  const rows: [string, string, number][] = [
    ["ok", "+", 58],
    ["ok", "+", 44],
    ["bad", "-", 52],
    ["plain", " ", 62],
    ["ok", "+", 38],
  ];
  return (
    <>
      <rect className="gd-sheet" x="22" y="38" width="100" height="92" rx="9" filter={`url(#${p}-shadow)`} />
      <T x={32} y={54} tone="ink">fix.diff</T>
      <line className="gd-rule" x1="22" y1="61" x2="122" y2="61" />
      {rows.map(([tone, sign, w], i) => (
        <g key={i}>
          <T x={32} y={76 + i * 11} tone={tone}>
            {sign}
          </T>
          <rect className={`gd-bar gd-bar--${tone}`} x={42} y={71 + i * 11} width={w} height={5} rx={2.5} />
        </g>
      ))}
      <Flow d="M122 84 L136 84" />
      <Flow d="M184 84 C196 84, 194 66, 206 66" />
      <Flow d="M184 84 C196 84, 194 102, 206 102" />
      <Hub p={p} x={160} y={84} />
      <Pill x={206} y={66} label="patch applied" tone="accent" anchor="start" />
      <Pill x={206} y={102} label="tests pass" tone="ok" dot anchor="start" />
    </>
  );
}

function Run({ p }: { p: string }) {
  const files: [string, number][] = [
    ["Cargo.toml", 52],
    ["package.json", 84],
    ["requirements.txt", 116],
  ];
  return (
    <>
      {files.map(([f, y]) => {
        const end = 14 + pillWidth(f, 10, false);
        return <Flow key={f} d={`M${end} ${y} C${end + 18} ${y}, 124 84, 144 84`} on={y === 84} />;
      })}
      {files.map(([f, y]) => (
        <Pill key={f} x={14} y={y} label={f} tone={y === 84 ? "accent" : "plain"} anchor="start" />
      ))}
      <Flow d="M192 84 L210 84" />
      <Hub p={p} x={168} y={84} />
      <Pill x={168} y={146} label="120 s default" />
      <rect className="gd-term" x="210" y="48" width="96" height="72" rx="9" />
      <Dots x={221} y={60} />
      <T x={220} y={82} tone="light" size={9}>
        $ npm test
      </T>
      <T x={220} y={98} tone="okdark" size={9}>
        ✓ passed
      </T>
      <T x={220} y={112} tone="dim" size={8}>
        vm deleted
      </T>
    </>
  );
}

function IssueText({ p }: { p: string }) {
  const bars: [number, number, boolean][] = [
    [64, 72, false],
    [76, 60, false],
    [88, 80, true],
    [100, 66, false],
    [112, 48, false],
  ];
  return (
    <>
      <rect className="gd-sheet" x="22" y="34" width="106" height="100" rx="9" filter={`url(#${p}-shadow)`} />
      <T x={32} y={50} tone="ink">issue #204</T>
      <line className="gd-rule" x1="22" y1="57" x2="128" y2="57" />
      {bars.map(([y, w, hot]) => (
        <rect key={y} className={hot ? "gd-bar gd-bar--hot" : "gd-bar"} x={32} y={y} width={w} height={5} rx={2.5} />
      ))}
      <Flow d="M128 84 L148 84" />
      <Flow d="M184 84 L214 84" on={false} />
      <Tile p={p} x={166} y={84} Icon={ScanText} soft />
      <T x={166} y={124} anchor="middle">scan_injection</T>
      <Pill x={214} y={84} label="BLOCKED" tone="bad" dot anchor="start" />
    </>
  );
}

function Manifest({ p }: { p: string }) {
  const lines: [string, boolean][] = [
    ["{", false],
    ['  "scripts": {', false],
    ['    "postinstall":', true],
    ['      "node x.js"', false],
    ["  }", false],
  ];
  return (
    <>
      <rect className="gd-sheet" x="20" y="30" width="128" height="108" rx="9" filter={`url(#${p}-shadow)`} />
      <T x={30} y={46} tone="ink">package.json</T>
      <line className="gd-rule" x1="20" y1="53" x2="148" y2="53" />
      {lines.map(([l, hot], i) =>
        hot ? (
          <g key={i}>
            <rect className="gd-hl" x={26} y={66 + i * 13 - 9.5} width={116} height={13} rx={3} />
            <T x={30} y={66 + i * 13} tone="accent">
              {l}
            </T>
          </g>
        ) : (
          <T key={i} x={30} y={66 + i * 13} tone="ink">
            {l}
          </T>
        ),
      )}
      <Flow d="M148 84 L168 84" />
      <Tile p={p} x={186} y={84} Icon={PackageSearch} soft />
      <Flow d="M204 84 L214 84" on={false} />
      <T x={186} y={124} anchor="middle">audit_supply</T>
      <Pill x={214} y={84} label="hook refused" tone="bad" dot anchor="start" />
    </>
  );
}

function Scrub() {
  return (
    <>
      <rect className="gd-term" x="22" y="30" width="276" height="108" rx="10" />
      <Dots x={35} y={43} />
      <T x={36} y={66} tone="dim" size={9}>
        $ cat ci.log
      </T>
      <T x={36} y={86} tone="light" size={9}>
        AWS_ACCESS_KEY_ID=
      </T>
      <Pill x={36 + 18 * 9 * MONO_ADVANCE + 7} y={83} label="[REDACTED]" tone="hot" size={9} anchor="start" />
      <T x={36} y={106} tone="light" size={9}>
        Authorization: Bearer
      </T>
      <Pill x={36 + 22 * 9 * MONO_ADVANCE + 4} y={103} label="[REDACTED]" tone="hot" size={9} anchor="start" />
      <T x={36} y={126} tone="okdark" size={9}>
        ✓ build passed
      </T>
    </>
  );
}

function Audit({ p }: { p: string }) {
  const cx0 = 160;
  const cy0 = 150;
  const R = 112;
  const nodes = [166, 137, 108, 72, 43, 14].map((a, i) => ({
    i,
    x: cx0 + R * Math.cos((a * Math.PI) / 180),
    y: cy0 - R * Math.sin((a * Math.PI) / 180),
  }));
  return (
    <>
      <path className="bv-arc" d={`M${cx0 - R} ${cy0} A${R} ${R} 0 0 1 ${cx0 + R} ${cy0}`} />
      {nodes.map((n) => (
        <Flow key={n.i} d={`M${cx0} 112 L${n.x.toFixed(1)} ${(n.y + 11).toFixed(1)}`} on={n.i === 2} />
      ))}
      {nodes.map((n) => (
        <g key={n.i} className={n.i === 2 ? "gd-phase is-on" : "gd-phase"} transform={`translate(${(n.x - 13).toFixed(1)} ${(n.y - 11).toFixed(1)})`}>
          <rect width="26" height="22" rx="7" filter={n.i === 2 ? undefined : `url(#${p}-shadow)`} />
          <text x="13" y="14.5" fontSize="9" textAnchor="middle">
            {`0${n.i + 1}`}
          </text>
        </g>
      ))}
      <Hub p={p} x={cx0} y={134} s={44} />
    </>
  );
}

function DockerVsVm({ p }: { p: string }) {
  return (
    <>
      <T x={78} y={30} anchor="middle" tone="ink">
        local docker
      </T>
      {[34, 66, 98].map((x, i) => (
        <g key={x}>
          <path className="bv-track" d={`M${x + 13} 94 L${x + 13} 112`} />
          <g className="bv-tile" transform={`translate(${x} 66)`}>
            <rect width="26" height="28" rx="7" filter={`url(#${p}-shadow)`} />
            <rect className="gd-bar" x="7" y="9" width="12" height="3" rx="1.5" />
            <rect className="gd-bar" x="7" y="16" width={i === 1 ? 8 : 12} height="3" rx="1.5" />
          </g>
        </g>
      ))}
      <rect className="gd-kernel" x="24" y="112" width="110" height="16" rx="5" />
      <T x={79} y={123} anchor="middle" size={8}>
        host kernel
      </T>
      <Pill x={79} y={148} label="shared" tone="bad" dot />
      <line className="gd-divide" x1="160" y1="22" x2="160" y2="152" />
      <T x={242} y={30} anchor="middle" tone="ink">
        airgap microVM
      </T>
      <Hub p={p} x={242} y={84} />
      <path className="bv-track" d="M242 108 L242 112" />
      <rect className="gd-kernel gd-kernel--own" x="206" y="112" width="72" height="16" rx="5" />
      <T x={242} y={123} anchor="middle" size={8} tone="accent">
        own kernel
      </T>
      <Pill x={242} y={148} label="deleted on exit" tone="ok" dot />
    </>
  );
}

function Trifecta({ p }: { p: string }) {
  const A: [number, number] = [160, 40];
  const B: [number, number] = [70, 126];
  const C: [number, number] = [250, 126];
  const mid = (u: [number, number], v: [number, number]): [number, number] => [(u[0] + v[0]) / 2, (u[1] + v[1]) / 2];
  const edges: [[number, number], [number, number]][] = [
    [A, B],
    [A, C],
    [B, C],
  ];
  return (
    <>
      {edges.map(([u, v], i) => (
        <path key={i} className="bv-arc bv-arc--solid" d={`M${u[0]} ${u[1]} L${v[0]} ${v[1]}`} />
      ))}
      {edges.map(([u, v], i) => {
        const [mx, my] = mid(u, v);
        return (
          <g key={i} className="gd-cut" transform={`translate(${mx} ${my})`}>
            <circle r="7.5" />
            <path d="M-2.6 -2.6 L2.6 2.6 M2.6 -2.6 L-2.6 2.6" />
          </g>
        );
      })}
      <Tile p={p} x={A[0]} y={A[1]} Icon={KeyRound} />
      <Tile p={p} x={B[0]} y={B[1]} Icon={Globe} />
      <Tile p={p} x={C[0]} y={C[1]} Icon={Send} />
      <T x={184} y={43}>private data</T>
      <T x={B[0]} y={158} anchor="middle">
        untrusted input
      </T>
      <T x={C[0]} y={158} anchor="middle">
        exfil channel
      </T>
      <Pill x={160} y={104} label="trifecta broken" tone="accent" />
    </>
  );
}

function SupplyChain({ p }: { p: string }) {
  const rows: [string, number, boolean][] = [
    ["serde 1.0", 50, false],
    ["left-pad-x", 84, true],
    ["requests 2.32", 118, false],
  ];
  return (
    <>
      <line className="gd-gate" x1="170" y1="26" x2="170" y2="142" />
      {rows.map(([name, y, bad]) => {
        const end = 16 + pillWidth(name, 10, false);
        return bad ? <Flow key={name} d={`M${end} ${y} L152 ${y}`} /> : <Flow key={name} d={`M${end} ${y} L210 ${y}`} on={false} />;
      })}
      {rows.map(([name, y, bad]) => (
        <Pill key={name} x={16} y={y} label={name} tone={bad ? "accent" : "plain"} anchor="start" />
      ))}
      <Tile p={p} x={170} y={84} Icon={ShieldCheck} soft />
      <path className="bv-track" d="M188 84 L210 84" />
      <Pill x={210} y={50} label="installed" tone="ok" dot anchor="start" />
      <Pill x={210} y={84} label="hook blocked" tone="bad" dot anchor="start" />
      <Pill x={210} y={118} label="installed" tone="ok" dot anchor="start" />
    </>
  );
}

function Entropy() {
  const heights = [18, 26, 14, 22, 30, 20, 76, 82, 24, 16, 28, 20, 34, 72, 22, 18, 26, 14];
  const base = 128;
  const thr = 64;
  return (
    <>
      <line className="gd-base" x1="22" y1={base} x2="298" y2={base} />
      {heights.map((h, i) => {
        const hot = base - h < thr;
        return <rect key={i} className={hot ? "gd-bar gd-bar--hot" : "gd-bar gd-bar--mute"} x={30 + i * 15} y={base - h} width={9} height={h} rx={2.5} />;
      })}
      <line className="gd-gate" x1="22" y1={thr} x2="298" y2={thr} />
      <T x={22} y={thr - 6}>
        entropy threshold
      </T>
      <Pill x={30 + 6.5 * 15 + 4.5} y={32} label="[REDACTED]" tone="ink" size={9} />
      <Pill x={30 + 13 * 15 + 4.5} y={32} label="[REDACTED]" tone="ink" size={9} />
      <T x={22} y={148}>
        token entropy
      </T>
    </>
  );
}

function Playbook({ p }: { p: string }) {
  const files: [string, number][] = [
    [".cursorrules", 50],
    ["CLAUDE.md", 84],
    ["AGENTS.md", 118],
  ];
  return (
    <>
      {files.map(([f, y]) => {
        const end = 16 + pillWidth(f, 10, false);
        return <Flow key={f} d={`M${end} ${y} C${end + 20} ${y}, 112 84, 134 84`} on={y === 84} />;
      })}
      {files.map(([f, y]) => (
        <Pill key={f} x={16} y={y} label={f} anchor="start" />
      ))}
      <Hub p={p} x={158} y={84} />
      <Flow d="M182 84 C194 84, 192 60, 204 60" />
      <path className="bv-track" d="M182 84 C194 84, 192 108, 204 108" />
      <Pill x={204} y={60} label="airgap · auto" tone="ok" dot size={9} anchor="start" />
      <Pill x={204} y={108} label="Bash · ask first" tone="accent" dot size={9} anchor="start" />
    </>
  );
}

const COVERS: Record<string, (props: { p: string }) => ReactNode> = {
  "verify-pr": VerifyPr,
  "verify-patch": VerifyPatch,
  run: Run,
  "issue-text": IssueText,
  manifest: Manifest,
  scrub: Scrub,
  audit: Audit,
  "docker-vs-microvm": DockerVsVm,
  "prompt-injection-rce": Trifecta,
  "supply-chain-hardening": SupplyChain,
  "secret-scrubbing": Entropy,
  "team-playbook": Playbook,
};

// `idp` keeps gradient and filter ids unique when several covers share a page.
export function GuideCover({ slug, idp = slug }: { slug: string; idp?: string }) {
  const Art = COVERS[slug];
  const p = `gc-${idp}`;
  return (
    <svg className="gd-art" viewBox="0 0 320 168" aria-hidden="true" focusable="false">
      <Defs p={p} />
      {Art ? <Art p={p} /> : null}
    </svg>
  );
}
