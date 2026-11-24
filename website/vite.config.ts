import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const ROUTES = ["docs", "guides", "terms", "conditions"];

// Static hosts serve /docs from docs/index.html, so each client-side route gets its own copy of the shell;
// 404.html lets GitHub Pages fall back to the app for any other path.
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

// Served from the domain root (Vercel, www.useairgap.dev). To host under a sub-path instead,
// set base to it and every link built with at() follows.
export default defineConfig(() => ({
  base: "/",
  plugins: [react(), routeShells()],
  resolve: {
    alias: { "@": new URL("./src", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1") },
  },
  build: {
    target: "es2022",
  },
}));
