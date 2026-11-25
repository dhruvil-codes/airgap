import { StrictMode, type ComponentType } from "react";
import { renderToString } from "react-dom/server";
import App from "./App";
import { HomePage } from "./pages/Home";
import { DocsPage } from "./pages/Docs";
import { GuideDetailPage, GuidesPage } from "./pages/Guides";
import { GUIDE_META, guidePath } from "./guides/meta";
import { TermsPage } from "./pages/Terms";
import { ConditionsPage } from "./pages/Conditions";
import { AboutPage } from "./pages/About";
import { ContactPage } from "./pages/Contact";
import { PrivacyPage } from "./pages/Privacy";
import { NotFoundPage } from "./pages/NotFound";

export { agentFiles } from "./agent/files";
export { PAGES, SITE } from "./agent/knowledge";

const SSR_PAGES: Record<string, { title?: string; Page: ComponentType }> = {
  "/": { Page: HomePage },
  "/docs": { title: "Docs", Page: DocsPage },
  "/guides": { title: "Guides", Page: GuidesPage },
  "/about": { title: "About", Page: AboutPage },
  "/contact": { title: "Contact", Page: ContactPage },
  "/privacy": { title: "Privacy", Page: PrivacyPage },
  "/terms": { title: "Terms", Page: TermsPage },
  "/conditions": { title: "Conditions of use", Page: ConditionsPage },
  ...Object.fromEntries(
    GUIDE_META.map((g) => [guidePath(g.slug), { title: g.title, Page: () => <GuideDetailPage slug={g.slug} /> }]),
  ),
};

export function render(path: string) {
  const hit = SSR_PAGES[path];
  return renderToString(
    <StrictMode>
      <App Page={hit ? hit.Page : NotFoundPage} title={hit?.title} missing={!hit} />
    </StrictMode>,
  );
}
