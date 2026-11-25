import { useEffect, useRef, useState } from "react";
import { Tabs } from "@base-ui/react/tabs";
import { AnimatePresence, m, useReducedMotion } from "motion/react";
import { ArrowRight, ArrowUpRight, Check, Star } from "lucide-react";
import { AnimatedShinyText } from "@/components/ui/animated-shiny-text";
import { CLIENTS, INSTALL, LINKS, VERSION, at } from "../content";
import { Badge, CopyButton, EASE, Ticks } from "../lib";

export function InstallTabs() {
  const [value, setValue] = useState<string>(INSTALL[0].value);
  const current = INSTALL.find((i) => i.value === value) ?? INSTALL[0];
  return (
    <Tabs.Root className="install" value={value} onValueChange={(v) => setValue(String(v))}>
      <div className="install-bar">
        <Tabs.List className="seg" aria-label="Install method">
          {INSTALL.map((i) => (
            <Tabs.Tab key={i.value} value={i.value} className="seg-tab">
              {i.label}
            </Tabs.Tab>
          ))}
          <Tabs.Indicator className="seg-indicator" />
        </Tabs.List>
        <CopyButton value={current.command} label="Copy install command" />
      </div>
      <div className="install-cmd">
        <span className="install-prompt" aria-hidden="true">
          {current.prompt}
        </span>
        <AnimatePresence mode="popLayout" initial={false}>
          <m.code
            key={current.value}
            initial={{ opacity: 0, filter: "blur(4px)", y: 4 }}
            animate={{ opacity: 1, filter: "blur(0px)", y: 0 }}
            exit={{ opacity: 0, filter: "blur(4px)", y: -4 }}
            transition={{ duration: 0.22, ease: EASE }}
          >
            {current.command}
          </m.code>
        </AnimatePresence>
      </div>
    </Tabs.Root>
  );
}

const LOOP_WORDS = ["anything.", "the tests.", "npm install.", "that PR."];

function LoopWords() {
  const reduce = useReducedMotion();
  const [i, setI] = useState(0);
  const [widths, setWidths] = useState<number[]>([]);
  const sizers = useRef<(HTMLSpanElement | null)[]>([]);

  useEffect(() => {
    // getBoundingClientRect reports zoomed (visual) pixels while the width is applied as a CSS length that the
    // page zoom scales again, so convert back to layout pixels. Browsers without standardized zoom report 1.
    let frame = 0;
    const measure = () => {
      frame = 0;
      const root = document.getElementById("root");
      const scale = root?.offsetWidth ? root.getBoundingClientRect().width / root.offsetWidth : 1;
      setWidths(sizers.current.map((el) => (el ? el.getBoundingClientRect().width / scale : 0)));
    };
    const schedule = () => {
      if (!frame) frame = window.requestAnimationFrame(measure);
    };
    schedule();
    void document.fonts?.ready.then(schedule);
    window.addEventListener("resize", schedule, { passive: true });
    return () => {
      if (frame) window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", schedule);
    };
  }, []);

  useEffect(() => {
    if (reduce) return;
    const id = window.setInterval(() => setI((n) => (n + 1) % LOOP_WORDS.length), 2600);
    return () => window.clearInterval(id);
  }, [reduce]);

  const word = LOOP_WORDS[i];
  return (
    <m.span
      className="loop"
      aria-hidden="true"
      animate={widths[i] ? { width: widths[i] } : undefined}
      transition={{ duration: 0.55, ease: EASE }}
    >
      <span className="loop-sizers">
        {LOOP_WORDS.map((w, n) => (
          <span key={w} ref={(el) => void (sizers.current[n] = el)}>
            {w}
          </span>
        ))}
      </span>
      <AnimatePresence mode="popLayout" initial={false}>
        <m.span key={word} className="loop-word">
          {[...word].map((ch, n) => (
            <m.span
              key={n}
              className="loop-ch"
              initial={{ opacity: 0, y: "0.45em", filter: "blur(8px)" }}
              animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
              exit={{ opacity: 0, y: "-0.45em", filter: "blur(8px)" }}
              transition={{ duration: 0.42, delay: n * 0.022, ease: EASE }}
            >
              {ch === " " ? "\u00a0" : ch}
            </m.span>
          ))}
        </m.span>
      </AnimatePresence>
    </m.span>
  );
}
const SIGNALS = [
  { logo: at("/logos/mcp.svg"), text: "Listed in the official MCP Registry", href: LINKS.mcpRegistry },
  { logo: at("/logos/gmi.svg"), text: "Sandboxes run on GMI Cloud AgentBox v2", href: LINKS.agentbox },
  { logo: at("/logos/github.svg"), text: `v${VERSION} \u00b7 open source, MIT OR Apache-2.0`, href: LINKS.releases },
] as const;

function SignalPill() {
  const reduce = useReducedMotion();
  const [i, setI] = useState(0);

  useEffect(() => {
    if (reduce) return;
    const id = window.setInterval(() => setI((n) => (n + 1) % SIGNALS.length), 3000);
    return () => window.clearInterval(id);
  }, [reduce]);

  const s = SIGNALS[i];
  return (
    <a
      className="pill signal"
      href={s.href}
      target="_blank"
      rel="noopener noreferrer"
    >
      <AnimatePresence mode="popLayout" initial={false}>
        <m.span
          key={s.href}
          className="signal-inner"
          initial={{ opacity: 0, y: 12, filter: "blur(4px)" }}
          animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
          exit={{ opacity: 0, y: -12, filter: "blur(4px)" }}
          transition={{ duration: 0.4, ease: EASE }}
        >
          <img className="signal-logo" src={s.logo} alt="" width={16} height={16} />
          <AnimatedShinyText shimmerWidth={90}>{s.text}</AnimatedShinyText>
        </m.span>
      </AnimatePresence>
      <span className="signal-arrow" aria-hidden="true">
        <ArrowUpRight size={13} strokeWidth={1.8} />
      </span>
    </a>
  );
}
export function Hero() {
  return (
    <section className="hero" id="top" aria-labelledby="hero-title">
      <div className="frame hero-frame">
        <Ticks />
        <div className="hero-inner">
          <SignalPill />
          <h1 id="hero-title" className="h1">
            <span className="sr-only">Let your agent run anything. Just not on your machine.</span>
            <span aria-hidden="true">
              Let your agent run <LoopWords />
            </span>
            <br />
            <span className="muted shimmer" aria-hidden="true">
              Just not on your machine.
            </span>
          </h1>
          <p className="hero-lead">
            airgap moves your coding agent's installs, builds and tests into a disposable Linux microVM on GMI Cloud.
            It scans untrusted text first, audits manifests before install, and scrubs secrets from the logs.
          </p>
          <div id="install" className="hero-install">
            <InstallTabs />
          </div>
          <div className="hero-ctas">
            <a className="btn btn--accent" href={at(at("/docs"))}>
              Read the docs <ArrowRight size={15} strokeWidth={1.8} />
            </a>
            <a className="btn btn--ghost" href={LINKS.repo} target="_blank" rel="noopener noreferrer">
              <Star size={15} strokeWidth={1.8} className="star-ic" /> Star on GitHub
            </a>
          </div>
          <ul className="hero-proof" aria-label="What you get">
            {["MIT OR Apache-2.0", "8 MCP tools", "6 prebuilt targets", "Local scanners need no keys"].map((t) => (
              <li key={t}>
                <Check size={13} strokeWidth={2} /> {t}
              </li>
            ))}
          </ul>
        </div>
      </div>
      <div className="frame clients-frame clients-frame--single">
        <div className="clients">
          <Badge>Works with</Badge>
          <ul>
            {CLIENTS.map((c) => (
              <li key={c.name}>
                <a
                  className="client-pill"
                  href="#setup"
                  title={`Show the ${c.name} setup command`}
                  onClick={() => window.dispatchEvent(new CustomEvent("airgap:client", { detail: c.tab }))}
                >
                  <img src={c.logo} alt="" width={16} height={16} />
                  {c.name}
                </a>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}
