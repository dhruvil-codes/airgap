import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import { PAGES } from "./src/agent/knowledge";

const ROUTES = PAGES.filter((p) => p.path !== "/").map((p) => p.path.slice(1));

// Static hosts serve /docs from docs/index.html, so each client-side route gets its own copy of the shell;
// 404.html lets static hosts fall back to the app for any other path. scripts/prerender.mjs then fills each
// shell with the server-rendered page.
function routeShells(): Plugin {
  return {
    name: "airgap-route-shells",
    apply: "build",
    enforce: "post",
    generateBundle(_, bundle) {
      const shell = bundle["index.html"];
      if (!shell || shell.type !== "asset") return;
      for (const route of ROUTES) {
        this.emitFile({ type: "asset", fileName: `${route}/index.html`, source: shell.source });
      }
      this.emitFile({ type: "asset", fileName: "404.html", source: shell.source });
    },
  };
}

// vite preview falls back to the home shell for /docs; serve the route's own prerendered file like Vercel does,
// otherwise hydration runs against the wrong markup.
function previewRouteShells(): Plugin {
  return {
    name: "airgap-preview-route-shells",
    configurePreviewServer(server) {
      server.middlewares.use((req: { url?: string }, _res, next) => {
        const [pathname, query] = (req.url ?? "/").split("?");
        const route = pathname.replace(/^\/+|\/+$/g, "");
        if (ROUTES.includes(route)) req.url = `/${route}/index.html${query ? `?${query}` : ""}`;
        else if (route && !route.includes(".") && !route.startsWith("@")) req.url = "/404.html";
        next();
      });
    },
  };
}

// Served from the domain root (Vercel, www.useairgap.dev). To host under a sub-path instead,
// set base to it and every link built with at() follows.
export default defineConfig(({ isSsrBuild }) => ({
  base: "/",
  plugins: [react(), ...(isSsrBuild ? [] : [routeShells(), previewRouteShells()])],
  resolve: {
    alias: { "@": new URL("./src", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1") },
  },
  // Bundle dependencies into the prerender build so Node never has to resolve their CSS or browser-only entry points.
  ssr: { noExternal: true },
  build: {
    target: "es2022",
    copyPublicDir: !isSsrBuild,
  },
}));
