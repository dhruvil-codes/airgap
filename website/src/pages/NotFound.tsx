import { at } from "../content";
import { DocPage, type DocSection } from "./DocLayout";

const c = (s: string) => <code>{s}</code>;

const SECTIONS: DocSection[] = [
  {
    id: "where",
    title: "Where to go instead",
    body: (
      <ul className="doc-links">
        <li>
          <a href={at("/")}>Home</a>
        </li>
        <li>
          <a href={at("/docs")}>Docs</a>
        </li>
        <li>
          <a href={at("/guides")}>Guides</a>
        </li>
        <li>
          <a href={at("/sitemap.xml")}>Sitemap</a>
        </li>
      </ul>
    ),
  },
  {
    id: "agents",
    title: "For AI agents",
    body: (
      <p>
        Start from <a href={at("/llms.txt")}>{c("/llms.txt")}</a> or <a href={at("/index.md")}>{c("/index.md")}</a>. Search
        the docs with {c("GET /api/v1/search?q=...")}, or ask a question at {c("GET /ask?query=...")}.
      </p>
    ),
  },
];

export function NotFoundPage() {
  return <DocPage badge="404" title="Page not found" lead="There is nothing at this address. It may have moved, or the link may be wrong." sections={SECTIONS} />;
}
