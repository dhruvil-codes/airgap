import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./site.css";
import "./smooth";
import { StrictMode } from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import App from "./App";

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element in index.html");

const og = new URLSearchParams(window.location.search).has("og");
if (og) document.documentElement.dataset.og = "";
const app = (
  <StrictMode>
    <App og={og} />
  </StrictMode>
);

// Pages are prerendered at build time; hydrate them. The OG card and the dev server start from an empty root.
if (root.hasChildNodes() && !og) hydrateRoot(root, app);
else {
  root.replaceChildren();
  createRoot(root).render(app);
}
