import { useEffect, useRef, useState, type ReactNode } from "react";
import { AnimatePresence, motion, useInView, useReducedMotion, useScroll, useSpring, useTransform } from "motion/react";
import { Tooltip } from "@base-ui/react/tooltip";
import { Check, Copy, X } from "lucide-react";
import type { CodeLine } from "./content";

export const EASE: [number, number, number, number] = [0.23, 1, 0.32, 1];
export const SPRING = { type: "spring", stiffness: 380, damping: 32, mass: 0.9 } as const;

export function cx(...parts: (string | false | null | undefined)[]) {
  return parts.filter(Boolean).join(" ");
}

export function Mark({ size = 20 }: { size?: number }) {
  return (
    <svg className="mark" width={size} height={size} viewBox="8 8 48 48" fill="currentColor" aria-hidden="true">
      <path d="M35.4 10L52.6 20Q56 22 52.6 24L35.4 34Q32 36 28.6 34L11.4 24Q8 22 11.4 20L28.6 10Q32 8 35.4 10Z" />
      <path d="M11.4 32L28.6 42Q32 44 35.4 42L52.6 32Q56 30 56 33.8V38.8Q56 42 52.6 44L35.4 54Q32 56 28.6 54L11.4 44Q8 42 8 38.8V33.8Q8 30 11.4 32Z" />
    </svg>
  );
}
export function GithubIcon({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

export function ScrollProgress() {
  const { scrollYProgress } = useScroll();
  const scaleX = useSpring(scrollYProgress, { stiffness: 220, damping: 30, restDelta: 0.001 });
  return <motion.div className="scroll-bar" style={{ scaleX }} aria-hidden="true" />;
}

export function Ticks() {
  return (
    <span className="ticks" aria-hidden="true">
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}

export function External({
  href,
  className,
  children,
  label,
}: {
  href: string;
  className?: string;
  children: ReactNode;
  label?: string;
}) {
  return (
    <a className={className} href={href} target="_blank" rel="noopener noreferrer" aria-label={label}>
      {children}
    </a>
  );
}

type CopyState = "idle" | "copied" | "failed";

export function CopyButton({ value, label = "Copy" }: { value: string; label?: string }) {
  const [state, setState] = useState<CopyState>("idle");
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setState("copied");
    } catch {
      setState("failed");
    }
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setState("idle"), 1600);
  }

  const Glyph = state === "copied" ? Check : state === "failed" ? X : Copy;

  return (
    <Tooltip.Root>
      <Tooltip.Trigger
        closeOnClick={false}
        render={
          <button
            type="button"
            className="copy"
            aria-label={label}
            data-state={state}
            onClick={() => {
              void copy();
            }}
          />
        }
      >
        <AnimatePresence initial={false} mode="popLayout">
          <motion.span
            key={state}
            className="copy-glyph"
            initial={{ opacity: 0, scale: 0.5, filter: "blur(3px)" }}
            animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
            exit={{ opacity: 0, scale: 0.5, filter: "blur(3px)" }}
            transition={{ duration: 0.18, ease: EASE }}
          >
            <Glyph size={14} strokeWidth={1.8} />
          </motion.span>
        </AnimatePresence>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Positioner sideOffset={8}>
          <Tooltip.Popup className="tip">
            {state === "copied" ? "Copied" : state === "failed" ? "Copy failed" : label}
          </Tooltip.Popup>
        </Tooltip.Positioner>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}

export function Row({
  index,
  id,
  dark,
  labelledBy,
  className,
  children,
}: {
  index: string;
  id?: string;
  dark?: boolean;
  labelledBy?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className={cx("row", dark && "row--dark", className)} aria-labelledby={labelledBy}>
      <div className="frame">
        <div className="gutter" aria-hidden="true">
          <span>{index}</span>
        </div>
        <div className="cell">{children}</div>
        <div className="rail" aria-hidden="true" />
      </div>
    </section>
  );
}

export function Badge({ children }: { children: ReactNode }) {
  return (
    <span className="badge">
      <i aria-hidden="true" />
      {children}
    </span>
  );
}

export function SectionHead({
  id,
  badge,
  title,
  muted,
  lead,
  align = "center",
}: {
  id: string;
  badge: string;
  title: string;
  muted?: string;
  lead?: ReactNode;
  align?: "center" | "start";
}) {
  return (
    <Reveal className={cx("head", align === "start" && "head--start")}>
      <Badge>{badge}</Badge>
      <h2 id={id} className="h2">
        {title}
        {muted && (
          <>
            <br />
            <span className="muted">{muted}</span>
          </>
        )}
      </h2>
      {lead && <p className="lead">{lead}</p>}
    </Reveal>
  );
}

export function Reveal({
  children,
  className,
  delay = 0,
  y = 18,
  as = "div",
}: {
  children: ReactNode;
  className?: string;
  delay?: number;
  y?: number;
  as?: "div" | "li" | "article";
}) {
  const reduce = useReducedMotion();
  const Comp = as === "li" ? motion.li : as === "article" ? motion.article : motion.div;
  return (
    <Comp
      className={className}
      initial={reduce ? false : { opacity: 0, y, filter: "blur(5px)" }}
      whileInView={{ opacity: 1, y: 0, filter: "blur(0px)" }}
      viewport={{ once: true, margin: "0px 0px -8% 0px" }}
      transition={{ duration: 0.62, delay, ease: EASE }}
    >
      {children}
    </Comp>
  );
}

export function ScrollFloat({
  children,
  className,
  offset = 24,
}: {
  children: ReactNode;
  className?: string;
  offset?: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const reduce = useReducedMotion();
  const { scrollYProgress } = useScroll({
    target: ref,
    offset: ["start end", "end start"],
  });
  const y = useTransform(scrollYProgress, [0, 1], reduce ? [0, 0] : [offset, -offset]);
  return (
    <motion.div ref={ref} className={className} style={{ y }}>
      {children}
    </motion.div>
  );
}

export function NumberRoll({ value }: { value: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  const inView = useInView(ref, { once: true, margin: "0px 0px -10% 0px" });
  return (
    <span ref={ref} className="roll" aria-label={value}>
      {[...value].map((ch, i) => {
        const d = Number(ch);
        if (Number.isNaN(d)) {
          return (
            <span key={i} className="roll-static" aria-hidden="true">
              {ch}
            </span>
          );
        }
        return (
          <span key={i} className="roll-window" aria-hidden="true">
            <motion.span
              className="roll-col"
              initial={{ y: "-90%" }}
              animate={{ y: inView ? `${-d * 10}%` : "-90%" }}
              transition={{ duration: 0.85, delay: 0.06 + i * 0.06, ease: EASE }}
            >
              {Array.from({ length: 10 }, (_, n) => (
                <span key={n}>{n}</span>
              ))}
            </motion.span>
          </span>
        );
      })}
    </span>
  );
}

export function Code({ lines, numbered = false }: { lines: readonly CodeLine[]; numbered?: boolean }) {
  return (
    <pre className={cx("code", numbered && "code--numbered")}>
      <code>
        {lines.map((line, i) => (
          <span className="code-line" key={i}>
            {numbered && <span className="code-n">{i + 1}</span>}
            <span>
              {line.map((seg, j) =>
                typeof seg === "string" ? (
                  <span key={j}>{seg}</span>
                ) : (
                  <span key={j} className={`tok tok--${seg[0]}`}>
                    {seg[1]}
                  </span>
                ),
              )}
            </span>
          </span>
        ))}
      </code>
    </pre>
  );
}
