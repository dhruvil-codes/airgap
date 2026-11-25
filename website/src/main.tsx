import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./site.css";
import { StrictMode, type ComponentType } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import App, { routeOf } from "./App";
import { GUIDE_META, guidePath } from "./guides/meta";

type ResolvedRoute = { Page: ComponentType; title?: string; missing?: boolean; bare?: boolean };

async function loadRoute(path: string, og: boolean): Promise<ResolvedRoute> {
  if (og) {
    const { OgImage } = await import("./sections/OgImage");
    return { Page: OgImage, bare: true };
  }
  if (path === "/") {
    const { HomePage } = await import("./pages/Home");
    return { Page: HomePage };
  }
  if (path === "/docs") {
    const { DocsPage } = await import("./pages/Docs");
    return { Page: DocsPage, title: "Docs" };
  }
  if (path === "/guides") {
    const { GuidesPage } = await import("./pages/Guides");
    return { Page: GuidesPage, title: "Guides" };
  }
  const guide = GUIDE_META.find((g) => guidePath(g.slug) === path);
  if (guide) {
    const { GuideDetailPage } = await import("./pages/Guides");
    return { Page: () => <GuideDetailPage slug={guide.slug} />, title: guide.title };
  }
  if (path === "/about") {
    const { AboutPage } = await import("./pages/About");
    return { Page: AboutPage, title: "About" };
  }
  if (path === "/contact") {
    const { ContactPage } = await import("./pages/Contact");
    return { Page: ContactPage, title: "Contact" };
  }
  if (path === "/privacy") {
    const { PrivacyPage } = await import("./pages/Privacy");
    return { Page: PrivacyPage, title: "Privacy" };
  }
  if (path === "/terms") {
    const { TermsPage } = await import("./pages/Terms");
    return { Page: TermsPage, title: "Terms" };
  }
  if (path === "/conditions") {
    const { ConditionsPage } = await import("./pages/Conditions");
    return { Page: ConditionsPage, title: "Conditions of use" };
  }
  const { NotFoundPage } = await import("./pages/NotFound");
  return { Page: NotFoundPage, missing: true };
}

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element in index.html");

const og = new URLSearchParams(window.location.search).has("og");
if (og) document.documentElement.dataset.og = "";
const path = routeOf(window.location.pathname);

void loadRoute(path, og).then((route) => {
  const app = (
    <StrictMode>
      <App {...route} />
    </StrictMode>
  );
  // Pages are prerendered at build time; hydrate them. The OG card and the dev server start from an empty root.
  if (root.hasChildNodes() && !og) hydrateRoot(root, app);
  else {
    root.replaceChildren();
    createRoot(root).render(app);
  }
});
