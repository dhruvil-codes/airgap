import { useEffect, useState, type ReactNode } from "react";
import { Badge, CopyButton, Reveal, Ticks, cx } from "../lib";

export type DocSection = { id: string; title: string; body: ReactNode };

export function Cmd({ children, copy }: { children: string; copy?: boolean }) {
  return (
    <div className="doc-code">
      <pre>
        <code>{children}</code>
      </pre>
      {copy !== false && (
        <span className="doc-code-copy">
          <CopyButton
            value={children
              .split("\n")
              .filter((l) => !l.trim().startsWith("#"))
              .join("\n")}
            label="Copy"
          />
        </span>
      )}
    </div>
  );
}

export function Prompt({ children }: { children: string }) {
  return (
    <div className="doc-prompt">
      <span aria-hidden="true">&gt;</span>
      <code>{children}</code>
    </div>
  );
}

export function DocTable({ head, rows }: { head: string[]; rows: ReactNode[][] }) {
  return (
    <div className="doc-table">
      <table>
        <thead>
          <tr>
            {head.map((h) => (
              <th key={h}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              {r.map((c, j) => (
                <td key={j}>{c}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function DocPage({
  badge,
  title,
  lead,
  updated,
  sections,
}: {
  badge: string;
  title: string;
  lead: string;
  updated?: string;
  sections: DocSection[];
}) {
  const [current, setCurrent] = useState(sections[0]?.id);

  useEffect(() => {
    const els = sections.map((s) => document.getElementById(s.id)).filter(Boolean) as HTMLElement[];
    const io = new IntersectionObserver(
      (entries) => {
        const hit = entries.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
        if (hit) setCurrent(hit.target.id);
      },
      { rootMargin: "-80px 0px -65% 0px" },
    );
    els.forEach((el) => io.observe(el));
    return () => io.disconnect();
  }, [sections]);

  return (
    <>
      <section className="page-hero" id="top" aria-labelledby="page-title">
        <div className="frame page-hero-frame">
          <Ticks />
          <Reveal className="page-hero-inner">
            <Badge>{badge}</Badge>
            <h1 id="page-title" className="h2">
              {title}
            </h1>
            <p className="lead">{lead}</p>
            {updated && <p className="page-updated">Last updated {updated}</p>}
          </Reveal>
        </div>
      </section>
      <section className="row doc-row">
        <div className="frame">
          <div className="gutter" aria-hidden="true" />
          <div className="cell doc-cell">
            <div className="doc">
              <nav className="doc-toc" aria-label="On this page">
                <p className="doc-toc-label">On this page</p>
                <ul>
                  {sections.map((s) => (
                    <li key={s.id}>
                      <a href={`#${s.id}`} className={cx(current === s.id && "is-current")}>
                        {s.title}
                      </a>
                    </li>
                  ))}
                </ul>
              </nav>
              <article className="prose">
                {sections.map((s) => (
                  <section key={s.id} id={s.id} className="prose-sec">
                    <h2>
                      <a href={`#${s.id}`}>{s.title}</a>
                    </h2>
                    {s.body}
                  </section>
                ))}
              </article>
            </div>
          </div>
          <div className="rail" aria-hidden="true" />
        </div>
      </section>
    </>
  );
}
