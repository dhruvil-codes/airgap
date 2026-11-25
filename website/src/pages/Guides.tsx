import { useEffect, type ReactNode } from "react";
import { ArrowLeft, ArrowUpRight } from "lucide-react";
import { at } from "../content";
import { Badge, Reveal, Ticks } from "../lib";
import { GuideCover } from "../guides/Covers";
import { GUIDE_BODY } from "../guides/bodies";
import { GUIDE_GROUPS, GUIDE_META, guidePath, type GuideMeta } from "../guides/meta";

const indexOf = (slug: string) => GUIDE_META.findIndex((g) => g.slug === slug);
const fig = (slug: string) => String(indexOf(slug) + 1).padStart(2, "0");

function GuideCard({ guide, idp, delay = 0 }: { guide: GuideMeta; idp?: string; delay?: number }) {
  return (
    <Reveal as="article" className="gd-card" delay={delay}>
      <div className="gd-cover">
        <span className="gd-fig" aria-hidden="true">
          {fig(guide.slug)}
        </span>
        <ArrowUpRight className="gd-go" size={15} strokeWidth={1.8} aria-hidden="true" />
        <GuideCover slug={guide.slug} idp={idp} />
      </div>
      <div className="gd-copy">
        <h3>
          <a href={at(guidePath(guide.slug))}>{guide.title}</a>
        </h3>
        <p>{guide.card}</p>
      </div>
    </Reveal>
  );
}

function PageHero({ badge, title, lead, back }: { badge: string; title: string; lead: string; back?: boolean }) {
  return (
    <section className="page-hero" id="top" aria-labelledby="page-title">
      <div className="frame page-hero-frame">
        <Ticks />
        <Reveal className="page-hero-inner">
          {back && (
            <a className="gd-back" href={at("/guides")}>
              <ArrowLeft size={14} strokeWidth={1.8} aria-hidden="true" />
              All guides
            </a>
          )}
          <Badge>{badge}</Badge>
          <h1 id="page-title" className="h2">
            {title}
          </h1>
          <p className="lead">{lead}</p>
        </Reveal>
      </div>
    </section>
  );
}

function Body({ children }: { children: ReactNode }) {
  return (
    <section className="row doc-row">
      <div className="frame">
        <div className="gutter" aria-hidden="true" />
        <div className="cell doc-cell">{children}</div>
        <div className="rail" aria-hidden="true" />
      </div>
    </section>
  );
}

export function GuidesPage() {
  // The guides used to be sections of one page; keep old /guides#slug links working.
  useEffect(() => {
    const id = decodeURIComponent(window.location.hash.slice(1));
    if (id && GUIDE_META.some((g) => g.slug === id)) window.location.replace(at(guidePath(id)));
  }, []);

  return (
    <>
      <PageHero
        badge="Guides"
        title="Ask in plain words."
        lead="Walkthroughs and security guides for the jobs airgap does most: verifying pull requests, isolating microVM execution, defending against prompt injection, and stopping supply-chain attacks."
      />
      <Body>
        <div className="gd-wrap">
          <section className="gd-before" id="before" aria-labelledby="before-title">
            <div>
              <h2 id="before-title" className="gd-label">
                Before you start
              </h2>
              {GUIDE_BODY.before}
            </div>
          </section>
          {GUIDE_GROUPS.map((group) => {
            const items = GUIDE_META.filter((g) => g.group === group.id);
            return (
              <section key={group.id} className="gd-group" id={group.id} aria-labelledby={`${group.id}-title`}>
                <div className="gd-group-head">
                  <h2 id={`${group.id}-title`}>{group.title}</h2>
                  <p>{group.lead}</p>
                </div>
                <div className="gd-grid">
                  {items.map((g, i) => (
                    <GuideCard key={g.slug} guide={g} delay={(i % 3) * 0.05} />
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      </Body>
    </>
  );
}

export function GuideDetailPage({ slug }: { slug: string }) {
  const i = indexOf(slug);
  const guide = GUIDE_META[i];
  const group = GUIDE_GROUPS.find((g) => g.id === guide.group);
  const more = [GUIDE_META[(i + 1) % GUIDE_META.length], GUIDE_META[(i + 2) % GUIDE_META.length]];
  return (
    <>
      <PageHero badge={group?.title ?? "Guide"} title={guide.title} lead={guide.summary} back />
      <Body>
        <div className="gd-detail">
          <div className="gd-cover gd-cover--hero">
            <span className="gd-fig" aria-hidden="true">
              {fig(slug)}
            </span>
            <GuideCover slug={slug} idp={`${slug}-hero`} />
          </div>
          <article className="prose gd-prose">
            <section className="prose-sec" id={slug}>
              <h2 className="sr-only">{guide.title}</h2>
              {GUIDE_BODY[slug]}
            </section>
          </article>
          <section className="gd-more" aria-labelledby="more-title">
            <div className="gd-group-head">
              <h2 id="more-title">Keep reading</h2>
              <p>
                <a href={at("/guides")}>All {GUIDE_META.length} guides</a>
              </p>
            </div>
            <div className="gd-grid gd-grid--2">
              {more.map((g) => (
                <GuideCard key={g.slug} guide={g} idp={`${g.slug}-more`} />
              ))}
            </div>
          </section>
        </div>
      </Body>
    </>
  );
}
