import { useEffect, type JSX } from "react";
import { Tooltip } from "@base-ui/react/tooltip";
import { Header } from "./sections/Header";
import { Hero } from "./sections/Hero";
import { BentoSection } from "./sections/Bento";
import { BoundarySection, FaqSection, GatesSection, SetupSection } from "./sections/Sections";
import { CtaBanner, Footer } from "./sections/Footer";
import { OgImage } from "./sections/OgImage";
import GradualBlur from "@/components/GradualBlur";
import { scrollToTarget } from "./smooth";
import { BASE } from "./content";
import { DocsPage } from "./pages/Docs";
import { GuidesPage } from "./pages/Guides";
import { TermsPage } from "./pages/Terms";
import { ConditionsPage } from "./pages/Conditions";
import { AboutPage } from "./pages/About";
import { ContactPage } from "./pages/Contact";
import { PrivacyPage } from "./pages/Privacy";
import { NotFoundPage } from "./pages/NotFound";

export const PAGES: Record<string, { title: string; Page: () => JSX.Element }> = {
  "/docs": { title: "Docs", Page: DocsPage },
  "/guides": { title: "Guides", Page: GuidesPage },
  "/about": { title: "About", Page: AboutPage },
  "/contact": { title: "Contact", Page: ContactPage },
  "/privacy": { title: "Privacy", Page: PrivacyPage },
  "/terms": { title: "Terms", Page: TermsPage },
  "/conditions": { title: "Conditions of use", Page: ConditionsPage },
};

export const routeOf = (pathname: string) => pathname.slice(BASE.length).replace(/\/+$/, "").replace(/\/index\.html$/, "") || "/";

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

// `path` is passed by the prerenderer; in the browser it comes from the URL so hydration renders the same route.
export default function App({ path = routeOf(window.location.pathname), og = false }: { path?: string; og?: boolean }) {
  const page = PAGES[path];
  const missing = !page && path !== "/";

  useEffect(() => {
    if (page) document.title = `${page.title} · airgap`;
    else if (missing) document.title = "Page not found · airgap";
    // Content mounts after the browser's own hash jump, so retry it once React has rendered.
    const id = decodeURIComponent(window.location.hash.slice(1));
    const el = id ? document.getElementById(id) : null;
    if (el) requestAnimationFrame(() => scrollToTarget(el, true));
  }, [page, missing]);

  if (og) {
    return (
      <OgImage />
    );
  }
  return (
    <Tooltip.Provider delay={250}>
      <a className="skip" href="#main">
        Skip to content
      </a>
      <Header />
      <main id="main">{page ? <page.Page /> : missing ? <NotFoundPage /> : <Home />}</main>
      <Footer />
      <GradualBlur target="page" position="bottom" height="4.5rem" strength={1.6} layers={6} curve="bezier" autoHide zIndex={30} />
    </Tooltip.Provider>
  );
}
