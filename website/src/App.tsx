import { useEffect, type JSX } from "react";
import { Tooltip } from "@base-ui/react/tooltip";
import { Header } from "./sections/Header";
import { Hero } from "./sections/Hero";
import { BentoSection } from "./sections/Bento";
import { BoundarySection, FaqSection, GatesSection, SetupSection } from "./sections/Sections";
import { CtaBanner, Footer, OgCard } from "./sections/Footer";
import GradualBlur from "@/components/GradualBlur";
import { scrollToTarget } from "./smooth";
import { BASE } from "./content";
import { DocsPage } from "./pages/Docs";
import { GuidesPage } from "./pages/Guides";
import { TermsPage } from "./pages/Terms";
import { ConditionsPage } from "./pages/Conditions";

const PAGES: Record<string, { title: string; Page: () => JSX.Element }> = {
  "/docs": { title: "Docs", Page: DocsPage },
  "/guides": { title: "Guides", Page: GuidesPage },
  "/terms": { title: "Terms", Page: TermsPage },
  "/conditions": { title: "Conditions of use", Page: ConditionsPage },
};

function Home() {
  return (
    <>
      <Hero />
      <BoundarySection />
      <BentoSection />
      <GatesSection />
      <SetupSection />
      <FaqSection />
      <CtaBanner />
    </>
  );
}

export default function App() {
  const path = window.location.pathname.slice(BASE.length).replace(/\/+$/, "") || "/";
  const page = PAGES[path];

  useEffect(() => {
    if (page) document.title = `${page.title} · airgap`;
    // Content mounts after the browser's own hash jump, so retry it once React has rendered.
    const id = decodeURIComponent(window.location.hash.slice(1));
    const el = id ? document.getElementById(id) : null;
    if (el) requestAnimationFrame(() => scrollToTarget(el, true));
  }, [page]);

  if (new URLSearchParams(window.location.search).has("og")) {
    return (
      <Tooltip.Provider>
        <div className="og-solo">
          <OgCard />
        </div>
      </Tooltip.Provider>
    );
  }
  return (
    <Tooltip.Provider delay={250}>
      <a className="skip" href="#main">
        Skip to content
      </a>
      <Header />
      <main id="main">{page ? <page.Page /> : <Home />}</main>
      <Footer />
      <GradualBlur target="page" position="bottom" height="4.5rem" strength={1.6} layers={6} curve="bezier" autoHide zIndex={30} />
    </Tooltip.Provider>
  );
}
